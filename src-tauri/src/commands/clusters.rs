use matc::clusters::{codec, dt_names};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandDto {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeDto {
    pub id: u32,
    pub name: String,
}

// Mirrors matc::clusters::codec::CommandField / FieldKind for Tauri serialisation.
// We re-serialise via serde so camelCase keys reach the frontend.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandFieldDto {
    pub tag: u32,
    pub name: String,
    pub kind: serde_json::Value,
    pub optional: bool,
    pub nullable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSchemaDto {
    pub fields: Vec<CommandFieldDto>,
}

#[tauri::command]
pub fn get_device_type_name(device_type: u32) -> Option<String> {
    dt_names::get_device_type_name(device_type).map(|s| s.to_string())
}

#[tauri::command]
pub fn list_cluster_attributes(cluster_id: u32) -> Vec<AttributeDto> {
    codec::get_attribute_list(cluster_id)
        .into_iter()
        .map(|(id, name)| AttributeDto {
            id,
            name: name.to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn list_cluster_commands(cluster_id: u32) -> Vec<CommandDto> {
    codec::get_command_list(cluster_id)
        .into_iter()
        .map(|(id, name)| CommandDto {
            id,
            name: name.to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn get_command_schema(cluster_id: u32, command_id: u32) -> Option<CommandSchemaDto> {
    let fields = codec::get_command_schema(cluster_id, command_id)?;
    let dto_fields = fields
        .into_iter()
        .map(|f| {
            let kind_json = serde_json::to_value(&f.kind).unwrap_or(serde_json::Value::Null);
            CommandFieldDto {
                tag: f.tag,
                name: f.name.to_string(),
                kind: kind_json,
                optional: f.optional,
                nullable: f.nullable,
            }
        })
        .collect();
    Some(CommandSchemaDto { fields: dto_fields })
}
