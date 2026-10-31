use anyhow::Result;
use jiff::Timestamp;

use super::{BoxFuture, FetchContext, WeatherProvider, WeatherReport};

/// Values from the configuration; for testing the display without network.
pub struct ManualProvider;

impl ManualProvider {
    pub const ID: &'static str = "manual";
}

impl WeatherProvider for ManualProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn display_name(&self) -> &'static str {
        "Manual / Test"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn fetch<'a>(&'a self, ctx: &'a FetchContext) -> BoxFuture<'a, Result<WeatherReport>> {
        let m = ctx.settings.manual.clone();
        Box::pin(async move {
            Ok(WeatherReport {
                condition: m.condition,
                is_day: m.is_day,
                current_c: m.current_c,
                max_c: m.max_c,
                min_c: m.min_c,
                observed_at: Timestamp::now(),
                provider: Self::ID.into(),
            })
        })
    }
}
