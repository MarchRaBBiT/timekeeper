use crate::config::{self, TimeZoneStatus};
use leptos::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConfigState {
    pub time_zone_status: TimeZoneStatus,
}

pub fn use_config() -> (ReadSignal<ConfigState>, WriteSignal<ConfigState>) {
    match use_context::<(ReadSignal<ConfigState>, WriteSignal<ConfigState>)>() {
        Some(ctx) => ctx,
        None => {
            let (read, write) = create_signal(ConfigState {
                time_zone_status: config::time_zone_status(),
            });
            provide_context((read, write));
            (read, write)
        }
    }
}

pub async fn refresh_time_zone(set_state: WriteSignal<ConfigState>) {
    set_state.update(|s| s.time_zone_status.loading = true);
    let status = config::refresh_time_zone().await;
    set_state.update(|s| s.time_zone_status = status);
}
