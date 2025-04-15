use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::process;
use fake::faker::lorem::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use rand::rng;
use rand::Rng;
use chrono;

#[derive(Deserialize, Debug, Clone)]
pub struct RouteStruct {
    pub path: String,
    pub no_of_entries: u64,
    pub schema: serde_json::Value,
    pub null_percentage: u8,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JsonDataGeneratorSchema {
    pub routes: Vec<RouteStruct>,
}

pub fn generate_json_from_schema(schema: JsonDataGeneratorSchema) -> Value {
    let mut result = serde_json::json!({});
    let mut rng_instance = rng();

    let routes = &schema.routes;
    if !routes.is_empty() {
        println!("🪨  Auto Generate ON, Found Schema file !!")
    } else {
        eprintln!("Please pass a schema file .json for your routes as `auto-generate-data` is enabled");
        process::exit(1);
    }

    for route in schema.routes {
        let mut route_data = Vec::new();
        let null_percentage = route.null_percentage;

        if !(null_percentage >= 0 && null_percentage<=90) {
            eprintln!("field `null_percentage` must be between 0 and 90 [including 0 and 90]");
            process::exit(1);
        } 

        for i in 0..route.no_of_entries {
            let mut entry = serde_json::Map::new();

            if let Some(schema_obj) = route.schema.as_object() {
                for (field_name, field_def) in schema_obj {
                    let roll = rng_instance.gen_range(0..100);
                    if roll < null_percentage {
                        entry.insert(field_name.clone(), json!(null));
                    } else {
                        let field_type = field_def.as_str().unwrap_or_else(|| infer_type_from_name(field_name));
                        let field_value = generate_value(field_name, field_type, field_def, i);
                        entry.insert(field_name.clone(), field_value);
                    }
                }
            }

            route_data.push(Value::Object(entry));
        }

        if let Some(obj) = result.as_object_mut() {
            obj.insert(route.path, Value::Array(route_data));
        }
    }

    result
}

fn generate_value(field_name: &str, _field_type: &str, field_def: &Value, mut index: u64) -> Value {
    let mut rng_instance = rng();

    let field_type = match field_def.as_str() {
        Some(ob) => ob,
        None => return json!(null),
    };

    match field_type {
        "string" | "name" | "lorem" | "integer" | "id" | "boolean" | "date" | "datetime" => {
            generate_faker_value(field_type, &mut rng_instance, index)
        }
        _ => Value::String(format!("Looks like we haven't added the category `{}`",field_type)),
    }
}

fn generate_faker_value(category: &str, rng: &mut impl Rng, mut index: u64) -> Value {
    match category {
        "name" => Value::String(Name().fake_with_rng::<String, _>(rng)),
        "lorem" => Value::String(Paragraph(1..3).fake_with_rng::<String, _>(rng)),
        "string" => Value::String(Word().fake_with_rng::<String, _>(rng)),
        "id" => {
            index = index + 1;
            let random_id: u64 = index;
            Value::Number(serde_json::Number::from(random_id))
        },
        "date" => {
            let date = chrono::Utc::now().format("%d-%m-%Y").to_string();
            Value::String(date)
        },
        "datetime" => {
            let datetime = chrono::Utc::now().to_rfc3339();
            Value::String(datetime)
        },
        "boolean" => Value::Bool(rng.gen_bool(0.5)),
        "integer" | "number" => {
            let num = rng.gen_range(1..100);
            Value::Number(serde_json::Number::from(num))
        }
        _ => Value::String(format!(
            "Looks like we haven't added the category `{}`",
            category
        )),
    }
}

fn infer_type_from_name(field_name: &str) -> &'static str {
    let lower_name = field_name.to_lowercase();
    
    if lower_name == "id" || lower_name.ends_with("_id") || lower_name.ends_with("id") {
        "integer"
    } else if lower_name.contains("date") || lower_name.contains("time") || lower_name == "created_on" || lower_name == "updated_on" {
        "date"
    } else if lower_name == "active" || lower_name == "enabled" || lower_name == "is_deleted" || lower_name.starts_with("is_") || lower_name.starts_with("has_") {
        "boolean"
    } else if lower_name == "count" || lower_name == "amount" || lower_name == "price" || lower_name == "quantity" {
        "number"
    } else {
        "string"
    }
}