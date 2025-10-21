use dioxus::prelude::*;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

mod database;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut db_status = use_signal(|| "Checking DB...".to_string());

    use_future(move || {
        let mut db_status = db_status.clone();
        async move {
            match database::db::get_db().await {
                Ok(db) => {
                    db_status.set("✅ Connected to database!".to_string());
                }
                Err(e) => db_status.set(format!("❌ Failed to connect: {e}")),
            }
        }
    });

    rsx! {
        div { class: "container mx-auto p-4",
            h2 { class: "text-xl font-bold mb-2", "Database Status" }
            p { class: "text-sm", "{db_status()}" }
        }
    }
}
