use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use transcription_core::{
    AccessToken, AuthorizationError, AuthorizationPort, Clock, ConnectivityPort,
};

pub struct UnavailableAuthorization;

#[async_trait]
impl AuthorizationPort for UnavailableAuthorization {
    async fn acquire(&self) -> Result<AccessToken, AuthorizationError> {
        Err(AuthorizationError::Unavailable)
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

pub struct OptimisticConnectivity;

#[async_trait]
impl ConnectivityPort for OptimisticConnectivity {
    async fn is_online(&self) -> bool {
        true
    }
}
