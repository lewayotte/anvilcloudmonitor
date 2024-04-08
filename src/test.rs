use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let mut map = HashMap::new();
    map.insert("lang", "rust");
    map.insert("body", "json");

    let res = client.post("http://httpbin.org/post")
        .json(&map)
        .send()
        .await?;

    let body = res.text().await?;
    println!("Response body: {:?}", body);

    Ok(())
}
