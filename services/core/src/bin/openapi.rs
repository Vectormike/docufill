fn main() {
    let document = docufill_core::api::openapi_document();
    println!(
        "{}",
        serde_json::to_string_pretty(&document).expect("OpenAPI document should serialize")
    );
}
