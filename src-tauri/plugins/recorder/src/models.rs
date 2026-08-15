use recorder_core::FinalizationReason;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopRequest {
    pub session_id: String,
    pub reason: FinalizationReason,
}

#[cfg(mobile)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeFinalizedRecording {
    pub artifact_id: String,
    pub session_id: String,
    pub file_uri: String,
    pub duration_ms: u64,
    pub byte_length: u64,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub sha256: String,
    pub finalization_reason: FinalizationReason,
}

#[cfg(test)]
mod tests {
    use recorder_core::{ArtifactId, FinalizationReason, FinalizedRecording, RecordingSessionId};

    use super::*;

    fn public_recording() -> FinalizedRecording {
        FinalizedRecording::new(
            ArtifactId::new(),
            RecordingSessionId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "audio/mp4",
            "m4a",
            500,
            128,
            44_100,
            1,
            "a".repeat(64),
            FinalizationReason::UserStop,
        )
        .unwrap()
    }

    #[test]
    fn public_recording_serialization_contains_no_file_locator() {
        let json = serde_json::to_string(&public_recording()).unwrap();
        assert!(!json.contains("fileUri"));
        assert!(!json.contains("/private/"));
        assert!(json.contains("artifactId"));
    }

    #[test]
    fn request_payloads_use_camel_case_and_stable_reason_values() {
        let request = StopRequest {
            session_id: "550e8400-e29b-41d4-a716-446655440000".into(),
            reason: FinalizationReason::UserStop,
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "sessionId": "550e8400-e29b-41d4-a716-446655440000",
                "reason": "userStop"
            })
        );
    }

    #[cfg(mobile)]
    #[test]
    fn native_locator_is_deserializable_but_not_part_of_public_type() {
        let native: NativeFinalizedRecording = serde_json::from_value(serde_json::json!({
            "artifactId": "c56a4180-65aa-42ec-a945-5fd21dec0538",
            "sessionId": "550e8400-e29b-41d4-a716-446655440000",
            "fileUri": "file:///private/app/Library/Application%20Support/Recordings/test.m4a",
            "durationMs": 500,
            "byteLength": 128,
            "sampleRateHz": 44100,
            "channelCount": 1,
            "sha256": "a".repeat(64),
            "finalizationReason": "userStop"
        }))
        .unwrap();
        assert!(native.file_uri.starts_with("file:"));
    }
}
