export const createTranscriptionContractDouble = ({ cleanup, provider, policy }) => {
  const reservations = new Map();
  const operations = new Map();
  const createCounts = new Map();
  const activeCounts = new Map();
  let operationSequence = 0;

  return {
    async submit(submission) {
      if (!submission.bearerPrincipal) {
        return {
          status: 401,
          body: {
            code: "AUTHENTICATION_REQUIRED",
            category: "user_actionable",
            retryable: false,
          },
        };
      }

      if (submission.dailyUsageAllowed === false) {
        return {
          status: 429,
          body: {
            code: "USAGE_LIMIT_EXCEEDED",
            category: "user_actionable",
            retryable: false,
          },
        };
      }

      const reservationKey = `${submission.bearerPrincipal}:${submission.idempotencyKey}`;
      const existing = reservations.get(reservationKey);

      if (existing) {
        if (existing.fingerprint !== submission.fingerprint) {
          return {
            status: 422,
            body: {
              code: "IDEMPOTENCY_MISMATCH",
              category: "terminal",
              retryable: false,
            },
          };
        }

        return {
          status: existing.current.state === "completed" ? 200 : 202,
          body: existing.current,
        };
      }

      const createCount = createCounts.get(submission.bearerPrincipal) ?? 0;
      if (createCount >= policy.limits.create_per_rolling_minute) {
        return {
          status: 429,
          body: {
            code: "RATE_LIMITED",
            category: "retryable",
            retryable: true,
          },
        };
      }

      const activeCount = activeCounts.get(submission.bearerPrincipal) ?? 0;
      if (activeCount >= policy.limits.active_operations_per_user) {
        return {
          status: 429,
          body: {
            code: "RATE_LIMITED",
            category: "retryable",
            retryable: true,
          },
        };
      }

      createCounts.set(submission.bearerPrincipal, createCount + 1);
      activeCounts.set(submission.bearerPrincipal, activeCount + 1);

      operationSequence += 1;
      const operationId = `operation-${operationSequence}`;
      const reservation = {
        fingerprint: submission.fingerprint,
        current: {
          id: operationId,
          source_audio_id: submission.sourceAudioId,
          state: "processing",
        },
        cancelled: false,
        completion: undefined,
      };
      reservation.completion = provider
        .transcribe({ operationId })
        .then((result) => {
          if (reservation.cancelled) return reservation.current;
          reservation.current = {
            ...reservation.current,
            state: "completed",
            result,
          };
          return reservation.current;
        })
        .finally(() => {
          const currentCount = activeCounts.get(submission.bearerPrincipal) ?? 0;
          activeCounts.set(submission.bearerPrincipal, Math.max(0, currentCount - 1));
        });

      reservations.set(reservationKey, reservation);
      operations.set(operationId, {
        owner: submission.bearerPrincipal,
        reservation,
      });

      return { status: 202, body: reservation.current };
    },
    async read({ bearerPrincipal, operationId }) {
      const stored = operations.get(operationId);
      if (!stored || stored.owner !== bearerPrincipal) {
        return {
          status: 404,
          body: {
            code: "OPERATION_NOT_FOUND",
            category: "terminal",
            retryable: false,
          },
        };
      }

      await stored.reservation.completion;
      return { status: 200, body: stored.reservation.current };
    },
    async delete({ bearerPrincipal, operationId }) {
      const stored = operations.get(operationId);
      if (!stored || stored.owner !== bearerPrincipal) {
        return {
          status: 404,
          body: {
            code: "OPERATION_NOT_FOUND",
            category: "terminal",
            retryable: false,
          },
        };
      }

      const { reservation } = stored;
      if (reservation.current.state === "deleted") return { status: 204 };

      reservation.cancelled = true;
      reservation.current = {
        ...reservation.current,
        state: "deleting",
        cleanup: {
          state: "in_progress",
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
