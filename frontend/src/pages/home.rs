use leptos::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-surface">
            <div class="max-w-7xl mx-auto py-12 px-4 sm:px-6 lg:px-8">
                <div class="text-center">
                    <h1 class="text-4xl font-extrabold text-fg sm:text-5xl lg:text-6xl">
                        "Timekeeper"
                    </h1>
                    <p class="mt-3 max-w-md mx-auto text-base text-fg-muted sm:text-lg lg:mt-5 lg:text-xl lg:max-w-3xl">
                        "少人数向けの勤怠管理システム"
                    </p>
                    <div class="mt-5 max-w-md mx-auto sm:flex sm:justify-center lg:mt-8">
                        <div class="rounded-md shadow">
                            <a href="/login" class="w-full flex items-center justify-center px-8 py-3 border border-transparent text-base font-medium rounded-md text-action-primary-text bg-action-primary-bg hover:bg-action-primary-bg_hover lg:py-4 lg:text-lg lg:px-10">
                                "ログイン"
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
