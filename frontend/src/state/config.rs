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
    use leptos::provide_context;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn use_config_seeds_time_zone_status() {
        with_runtime(|| {
            let _guard = config::acquire_test_serial_lock();
            let (read, _write) = use_config();
            let seeded = read.get().time_zone_status;
            assert_eq!(seeded, config::time_zone_status());
        });
    }

    #[test]
    fn use_config_prefers_existing_context() {
        with_runtime(|| {
            let _guard = config::acquire_test_serial_lock();
            let (ctx_read, ctx_write) = create_signal(ConfigState {
                time_zone_status: TimeZoneStatus {
                    time_zone: Some("Asia/Tokyo".into()),
                    is_fallback: false,
                    last_error: None,
                    loading: false,
                },
            });
            provide_context((ctx_read, ctx_write));

            let (read, write) = use_config();
            assert_eq!(read.get(), ctx_read.get());

            write.update(|s| s.time_zone_status.loading = true);
            assert!(ctx_read.get().time_zone_status.loading);
        });
    }

    #[test]
    fn refresh_time_zone_updates_state_from_config_result() {
        with_runtime(|| {
            let _guard = config::acquire_test_serial_lock();
            config::overwrite_time_zone_status_for_test(TimeZoneStatus {
                time_zone: Some("UTC".into()),
                is_fallback: true,
                last_error: Some("old error".into()),
                loading: false,
            });
            config::queue_mock_time_zone_fetch(Ok("Asia/Tokyo".into()));

            let (read, write) = create_signal(ConfigState {
                time_zone_status: config::time_zone_status(),
            });
            futures::executor::block_on(async {
                refresh_time_zone(write).await;
            });

            let status = read.get().time_zone_status;
            assert_eq!(status.time_zone.as_deref(), Some("Asia/Tokyo"));
            assert!(!status.is_fallback);
            assert!(status.last_error.is_none());
            assert!(!status.loading);
            assert_eq!(status, config::time_zone_status());
        });
    }
}
