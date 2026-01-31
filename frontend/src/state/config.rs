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

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::create_runtime;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn use_config_seeds_time_zone_status() {
        with_runtime(|| {
            let (read, _write) = use_config();
            let seeded = read.get().time_zone_status;
            assert_eq!(seeded, config::time_zone_status());
        });
    }
}
