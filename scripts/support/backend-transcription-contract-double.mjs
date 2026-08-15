export const createTranscriptionContractDouble = ({ cleanup, provider, policy, queue }) => {
  const reservations = new Map();
  const operations = new Map();
  const createCounts = new Map();
  const activeCounts = new Map();
  let operationSequence = 0;
  let requestSequence = 0;

  const timestamp = () => new Date().toISOString();
  const nextRequestId = () => {
    requestSequence += 1;
    return `request-${requestSequence}`;
  };
  const problem = (status, code, category, retryable) => ({
    type: `/problems/${code.toLowerCase().replaceAll("_", "-")}`,
    title: code
      .toLowerCase()
      .split("_")
      .map((word) => word[0].toUpperCase() + word.slice(1))
      .join(" "),
    status,
    code,
    category,
    retryable,
    request_id: nextRequestId(),
  });

  const releaseActiveSlot = (principal, reservation) => {
    if (!reservation.activeSlotHeld) return;
    reservation.activeSlotHeld = false;
    const currentCount = activeCounts.get(principal) ?? 0;
    activeCounts.set(principal, Math.max(0, currentCount - 1));
  };

  return {
    async submit(submission) {
      if (!submission.bearerPrincipal) {
        return {
          status: 401,
          body: problem(401, "AUTHENTICATION_REQUIRED", "user_actionable", false),
        };
      }

      if (submission.dailyUsageAllowed === false) {
        return {
          status: 429,
          body: problem(429, "USAGE_LIMIT_EXCEEDED", "user_actionable", false),
        };
      }

      const reservationKey = `${submission.bearerPrincipal}:${submission.idempotencyKey}`;
      const existing = reservations.get(reservationKey);

      if (existing) {
        if (existing.fingerprint !== submission.fingerprint) {
          return {
            status: 422,
            body: problem(422, "IDEMPOTENCY_MISMATCH", "terminal", false),
          };
        }

        return {
          status: ["completed", "failed", "cancelled", "deleted"].includes(existing.current.state)
            ? 200
            : 202,
          body: existing.current,
        };
      }

      const createCount = createCounts.get(submission.bearerPrincipal) ?? 0;
      if (createCount >= policy.limits.create_per_rolling_minute) {
        return {
          status: 429,
          body: problem(429, "RATE_LIMITED", "retryable", true),
        };
      }

      const activeCount = activeCounts.get(submission.bearerPrincipal) ?? 0;
      if (activeCount >= policy.limits.active_operations_per_user) {
        return {
          status: 429,
          body: problem(429, "RATE_LIMITED", "retryable", true),
        };
      }

      createCounts.set(submission.bearerPrincipal, createCount + 1);
      activeCounts.set(submission.bearerPrincipal, activeCount + 1);

      operationSequence += 1;
      const operationId = `operation-${operationSequence}`;
      const createdAt = timestamp();
      const reservation = {
        fingerprint: submission.fingerprint,
        current: {
          id: operationId,
          request_id: nextRequestId(),
          source_audio_id: submission.sourceAudioId,
          state: "queued",
          created_at: createdAt,
          updated_at: createdAt,
          cleanup: { state: "not_scheduled", content_available: true },
          links: { self: `/v1/transcriptions/${operationId}` },
        },
        cancelled: false,
        activeSlotHeld: true,
        completion: undefined,
      };
      let resolveCompletion;
      let rejectCompletion;
      reservation.completion = new Promise((resolve, reject) => {
        resolveCompletion = resolve;
        rejectCompletion = reject;
      });
      const dispatch = async () => {
        if (reservation.cancelled) {
          resolveCompletion(reservation.current);
          return reservation.current;
        }
        reservation.current = {
          ...reservation.current,
          state: "processing",
          updated_at: timestamp(),
        };
        try {
          const result = await provider.transcribe({ operationId });
          if (reservation.cancelled) {
            resolveCompletion(reservation.current);
            return reservation.current;
          }
          reservation.current = {
            ...reservation.current,
            state: "completed",
            updated_at: timestamp(),
            result,
            cleanup: {
              state: "scheduled",
              content_available: true,
              delete_by: new Date(
                Date.now() + policy.retention.terminal_content_delete_hours * 60 * 60 * 1_000,
              ).toISOString(),
            },
          };
          resolveCompletion(reservation.current);
          return reservation.current;
        } catch (error) {
          rejectCompletion(error);
          throw error;
        } finally {
          releaseActiveSlot(submission.bearerPrincipal, reservation);
        }
      };

      reservations.set(reservationKey, reservation);
      operations.set(operationId, {
        owner: submission.bearerPrincipal,
        reservation,
      });
      queue.dispatch(dispatch);

      return { status: 202, body: reservation.current };
    },
    async read({ bearerPrincipal, operationId }) {
      const stored = operations.get(operationId);
      if (!stored || stored.owner !== bearerPrincipal) {
        return {
          status: 404,
          body: problem(404, "OPERATION_NOT_FOUND", "terminal", false),
        };
      }

      if (stored.reservation.current.state === "processing") {
        await stored.reservation.completion;
      }
      return { status: 200, body: stored.reservation.current };
    },
    async delete({ bearerPrincipal, operationId }) {
      const stored = operations.get(operationId);
      if (!stored || stored.owner !== bearerPrincipal) {
        return {
          status: 404,
          body: problem(404, "OPERATION_NOT_FOUND", "terminal", false),
        };
      }

      const { reservation } = stored;
      if (reservation.current.state === "deleted") return { status: 204 };

      const wasQueued = reservation.current.state === "queued";
      reservation.cancelled = true;
      releaseActiveSlot(bearerPrincipal, reservation);
      reservation.current = {
        ...reservation.current,
        state: wasQueued ? "cancelled" : "deleting",
        updated_at: timestamp(),
        cleanup: {
          state: wasQueued ? "scheduled" : "in_progress",
          content_available: false,
          delete_by: new Date(
            Date.now() + policy.retention.terminal_content_delete_hours * 60 * 60 * 1_000,
          ).toISOString(),
        },
      };
      delete reservation.current.result;

      const outcome = await cleanup.remove({ operationId });
      if (outcome === "completed") {
        reservation.current = {
          ...reservation.current,
          state: "deleted",
          cleanup: { ...reservation.current.cleanup, state: "completed" },
        };
        return { status: 204 };
      }
      if (outcome === "failed") {
        reservation.current = {
          ...reservation.current,
          cleanup: { ...reservation.current.cleanup, state: "failed_retrying" },
        };
      }

      return { status: 202, body: reservation.current };
    },
  };
};
