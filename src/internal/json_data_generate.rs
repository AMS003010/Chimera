use chrono;
use fake::faker::lorem::en::*;
use fake::faker::name::en::*;
use fake::Fake;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Deserialize;
use serde_json::{json, Map, Number, Value};

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
    let mut rng = StdRng::from_rng(&mut rand::rng());
    let mut result = Map::new();

    if schema.routes.is_empty() {
        eprintln!("Schema file must contain at least one route when `auto-generate-data` is enabled");
        std::process::exit(1);
    }

    for route in schema.routes.iter() {
        if route.null_percentage > 90 {
            eprintln!("`null_percentage` for route `{}` must be between 0 and 90", route.path);
            std::process::exit(1);
        }

        let mut route_data = Vec::with_capacity(route.no_of_entries as usize);
        if let Some(schema_obj) = route.schema.as_object() {
            for i in 0..route.no_of_entries {
                let mut entry = Map::new();
                for (field_name, field_def) in schema_obj {
                    let value = if rng.random_range(0..100) < route.null_percentage {
                        json!(null)
                    } else {
                        let field_type = field_def.as_str().unwrap_or_else(|| infer_type_from_name(field_name));
                        generate_value(field_type, &mut rng, i)
                    };
                    entry.insert(field_name.clone(), value);
                }
                route_data.push(Value::Object(entry));
            }
        }
        result.insert(route.path.clone(), Value::Array(route_data));
    }

    Value::Object(result)
}

fn generate_value(field_type: &str, rng: &mut StdRng, index: u64) -> Value {
    match field_type {
        "name" => Value::String(Name().fake_with_rng(rng)),
        "lorem" => Value::String(Paragraph(1..3).fake_with_rng(rng)),
        "string" => Value::String(Word().fake_with_rng(rng)),
        "id" => Value::Number(Number::from(index + 1)),
        "date" => Value::String(chrono::Utc::now().format("%d-%m-%Y").to_string()),
        "datetime" => Value::String(chrono::Utc::now().to_rfc3339()),
        "boolean" => Value::Bool(rng.random_bool(0.5)),
        "integer" | "number" => Value::Number(Number::from(rng.random_range(1..100))),
        _ => Value::String("unsupported_type".to_string()),
    }
}

fn infer_type_from_name(field_name: &str) -> &'static str {
    let lower_name = field_name.to_lowercase();
    match lower_name.as_str() {
        "id" | "created_by_id" | "updated_by_id" => "id",
        name if name.ends_with("_id") || name.ends_with("id") => "id",
        name if name.contains("date") || name == "created_on" || name == "updated_on" => "date",
        name if name == "active" || name == "enabled" || name == "is_deleted" || name.starts_with("is_") || name.starts_with("has_") => "boolean",
        name if name == "count" || name == "amount" || name == "price" || name == "quantity" => "number",
        _ => "string",
    }
}