use reqwest::Client;
use serde_json::Value;
use rand::seq::SliceRandom;
use tokio::time::{Duration, timeout};


async fn create_tables()->Vec<i64>{
    let client = Client::new();
    let table_codes = vec!["T-01", "T-02", "T-03", "T-04", "T-05"];
    let mut table_ids = Vec::new();

    for index in 0..5 {
        let response: Value = client
            .post("http://localhost:3030/tables/create")
            .json(&serde_json::json!({"code": table_codes[index]}))
            .send()
            .await
            .expect("Failed to create table")
            .json()
            .await
            .expect("Failed to parse response");

        table_ids.push(response["id"].as_i64().expect("Missing or invalid id"));
    }

    return table_ids;
}

async fn create_menus()->Vec<i64>{
    let client = Client::new();
    let menu_names = ["Menu-01", "Menu-02", "Menu-03", "Menu-04", "Menu-05"];
    let mut menu_ids = Vec::new();

    for index in 0..5 {
        let response: Value = client
            .post("http://localhost:3030/menus/create")
            .json(&serde_json::json!({"name": menu_names[index]}))
            .send()
            .await
            .expect("Failed to create table")
            .json()
            .await
            .expect("Failed to parse response");

        menu_ids.push(response["id"].as_i64().expect("Missing or invalid id"));
    }

    return menu_ids;
}


async fn order_simulation(client: &Client, table_ids: &[i64], menu_ids: &[i64]) {
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let client = client.clone();
            let table_id = *table_ids.choose(&mut rand::thread_rng()).unwrap();
            let mut menu_subarray = menu_ids.to_vec();
            menu_subarray.shuffle(&mut rand::thread_rng());
            menu_subarray.truncate(3);
            tokio::spawn(async move {
                let response = client
                    .post("http://localhost:3030/orders/create")
                    .json(&serde_json::json!({
                        "table_id": table_id,
                        "menu_ids": menu_subarray,
                    }))
                    .send()
                    .await
                    .expect("Failed to create order")
                    .json::<serde_json::Value>()
                    .await
                    .expect("Failed to parse response");

                println!("Created Order for table {} with menus {:?}: {:?}", table_id, menu_subarray, response);
                tokio::time::sleep(Duration::from_secs(1)).await;

                let response = client
                    .get(&format!("http://localhost:3030/tables/{}/items", table_id))
                    .send()
                    .await
                    .expect("Failed to get all items")
                    .json::<serde_json::Value>()
                    .await
                    .expect("Failed to parse response");

                if let Some(items) = response.as_array() {
                    let mut new_array = Vec::new();
                
                    for item in items {
                        if let (Some(menu), Some(time), Some(quantity)) = (
                            item.get("menu_name").and_then(|v| v.as_str()),
                            item.get("cooking_time").and_then(|v| v.as_i64()),
                            item.get("quantity").and_then(|v| v.as_i64()),
                        ) {
                            let new_item = (menu, time, quantity);
                            new_array.push(new_item);
                        }
                    }
                
                    println!("All Items from Table {}: {:?}", table_id, new_array);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;

                if let Some(menu_id) = menu_subarray.first() {
                    let response = client
                        .get(&format!("http://localhost:3030/tables/{}/items/{}", table_id, *menu_id))
                        .send()
                        .await
                        .expect("Failed to get specific item")
                        .json::<serde_json::Value>()
                        .await
                        .expect("Failed to parse response");

                    println!("Menu {} from table {} is: Menu: {:?}, Cooking Time: {:?}, Quantity: {:?}", menu_id, table_id, response["menu_name"].as_str(), response["cooking_time"].as_i64(), response["quantity"].as_i64());
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }

                if let Some(menu_id) = menu_subarray.first() {
                    let response = client
                        .delete(&format!("http://localhost:3030/orders/{}/items/{}", table_id, *menu_id))
                        .send()
                        .await
                        .expect("Failed to remove item")
                        .json::<serde_json::Value>()
                        .await
                        .expect("Failed to parse response");

                    println!("Removed Menu {} from Table {}: {:?}", menu_id, table_id, response);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            })
        })
        .collect();

    for handle in handles {
        if let Err(e) = timeout(Duration::from_secs(30), handle).await {
            eprintln!("Task timed out: {:?}", e);
        }
    }
}

#[tokio::main]
async fn main() {


    let table_ids = create_tables().await;
    let menu_ids = create_menus().await;

    let client = Client::new();
    order_simulation(&client, &table_ids, &menu_ids).await

}