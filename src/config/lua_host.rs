use super::settings::{Config, Titles};
use mlua::{Lua, Table, Value};
use std::path::Path;

const TITLE_KEYS: [&str; 4] = ["filter", "queue", "mini", "info"];

const OPTION_KEYS: [&str; 4] = [
    "mpv_binary",
    "volume_step",
    "seek_step",
    "enter_resume_loaded",
];

const BOOTSTRAP: &str = r#"
local options_meta = {
    __newindex = function(t, k, v)
        if k == "titles" then
            if type(v) ~= "table" then
                error("invalid config:\nvmp.opt.titles: expected table or field assignments like vmp.opt.titles.queue = \"Queue\"", 0)
            end
            local allowed = { filter = true, queue = true, mini = true, info = true }
            for tk, tv in pairs(v) do
                if not allowed[tk] then
                    error("invalid config:\nvmp.opt.titles: unknown title '" .. tostring(tk) .. "'", 0)
                end
                if type(tv) ~= "string" then
                    error("invalid config:\nvmp.opt.titles." .. tostring(tk) .. ": expected string", 0)
                end
            end
            rawset(t, "titles", v)
            return
        end
        if rawget(vmp, "known_options")[k] == nil then
            error("invalid config:\nvmp.opt: unknown option '" .. tostring(k) .. "'", 0)
        end
        rawset(t, k, v)
    end,
}

local map_meta = {
    __newindex = function(t, k, v)
        if rawget(vmp, "known_actions")[k] == nil then
            error("invalid config:\nvmp.map: unknown action '" .. tostring(k) .. "'", 0)
        end
        if type(v) == "string" then
            rawset(t, k, v)
            return
        end
        if type(v) == "table" then
            for _, kv in ipairs(v) do
                if type(kv) ~= "string" then
                    error("invalid config:\nvmp.map." .. tostring(k) .. ": every key must be a string", 0)
                end
            end
            rawset(t, k, v)
            return
        end
        error("invalid config:\nvmp.map." .. tostring(k) .. ": expected a key string or a list of key strings", 0)
    end,
}

vmp = {
    known_options = {},
    known_actions = {},
    opt = setmetatable({}, options_meta),
    map = setmetatable({}, map_meta),
}
"#;

pub(super) fn eval_config(raw: &str, lua_dir: &Path) -> Result<Config, String> {
    let lua = Lua::new();
    let globals = lua.globals();
    let package: Table = globals
        .get("package")
        .map_err(|e| format!("invalid config: lua runtime: {e}"))?;
    let escaped = lua_dir.to_string_lossy();
    package
        .set("path", format!("{escaped}/?.lua;{escaped}/?/init.lua"))
        .map_err(|e| e.to_string())?;
    package.set("cpath", "").map_err(|e| e.to_string())?;

    lua.load(BOOTSTRAP)
        .set_name("vmp-bootstrap")
        .exec()
        .map_err(|e| exec_error_text(&e))?;

    let vmp: Table = globals.get("vmp").map_err(|e| e.to_string())?;
    let options: Table = vmp.get("known_options").map_err(|e| e.to_string())?;
    for key in OPTION_KEYS {
        options.set(key, true).map_err(|e| e.to_string())?;
    }
    options.set("titles", true).map_err(|e| e.to_string())?;
    let actions: Table = vmp.get("known_actions").map_err(|e| e.to_string())?;
    for (action, _) in crate::app::action::ACTIONS {
        actions
            .set(crate::app::action::action_str(*action), true)
            .map_err(|e| e.to_string())?;
    }

    seed_opt(&lua, &vmp).map_err(|e| e.to_string())?;

    lua.load(raw)
        .set_name("init.lua")
        .exec()
        .map_err(|e| exec_error_text(&e))?;

    harvest(&lua, &vmp)
}

fn exec_error_text(e: &mlua::Error) -> String {
    let text = e.to_string();
    let base = text.split("\nstack traceback:").next().unwrap_or(&text);
    let base = base.strip_prefix("runtime error: ").unwrap_or(base);
    if base.starts_with("invalid config:") {
        base.to_string()
    } else {
        format!("invalid config: {base}")
    }
}

fn seed_opt(lua: &Lua, vmp: &Table) -> Result<(), mlua::Error> {
    let opt: Table = vmp.get("opt")?;
    opt.raw_set("mpv_binary", "mpv")?;
    opt.raw_set("volume_step", 5)?;
    opt.raw_set("seek_step", 5.0)?;
    opt.raw_set("enter_resume_loaded", true)?;
    let titles = lua.create_table()?;
    titles.set("filter", "Filter")?;
    titles.set("queue", "Queue")?;
    titles.set("mini", "Mini Queue")?;
    titles.set("info", "Info")?;
    opt.raw_set("titles", titles)?;
    Ok(())
}

fn harvest(lua: &Lua, vmp: &Table) -> Result<Config, String> {
    let _ = lua;
    let opt: Table = vmp.get("opt").map_err(|e| e.to_string())?;
    let mpv_binary = string_opt(&opt, "mpv_binary")?;
    let volume_step = step_opt(&opt, "volume_step")?;
    let seek_step = number_opt(&opt, "seek_step")?;
    let enter_resume_loaded = boolean_opt(&opt, "enter_resume_loaded")?;
    let titles_value: Value = opt.raw_get("titles").map_err(|e| e.to_string())?;
    let titles = harvest_titles(titles_value)?;
    let map: Table = vmp.get("map").map_err(|e| e.to_string())?;
    let mut keybindings = harvest_map(&map)?;
    crate::config::validation::merge_and_validate_bindings(&mut keybindings)?;
    Ok(Config {
        mpv_binary,
        volume_step,
        seek_step,
        enter_resume_loaded,
        titles,
        keybindings,
    })
}

fn string_opt(opt: &Table, key: &str) -> Result<String, String> {
    match opt.raw_get::<Value>(key) {
        Ok(Value::String(s)) => Ok(s.to_str().map_err(|e| e.to_string())?.to_string()),
        Ok(other) => Err(format!(
            "invalid config:\n{key}: expected string, got lua type {}",
            value_name(&other)
        )),
        Err(e) => Err(format!("invalid config:\n{key}: {e}")),
    }
}

fn number_opt(opt: &Table, key: &str) -> Result<f64, String> {
    match opt.raw_get::<Value>(key) {
        Ok(Value::Number(n)) => Ok(n),
        Ok(Value::Integer(i)) => Ok(i as f64),
        Ok(other) => Err(format!(
            "invalid config:\n{key}: expected number, got lua type {}",
            value_name(&other)
        )),
        Err(e) => Err(format!("invalid config:\n{key}: {e}")),
    }
}

fn step_opt(opt: &Table, key: &str) -> Result<i32, String> {
    match opt.raw_get::<Value>(key) {
        Ok(Value::Integer(i)) => Ok(i as i32),
        Ok(Value::Number(n)) if n.is_finite() && n.fract() == 0.0 => Ok(n as i32),
        Ok(other) => Err(format!(
            "invalid config:\n{key}: expected integer, got {}",
            value_name(&other)
        )),
        Err(e) => Err(format!("invalid config:\n{key}: {e}")),
    }
}

fn boolean_opt(opt: &Table, key: &str) -> Result<bool, String> {
    match opt.raw_get::<Value>(key) {
        Ok(Value::Boolean(b)) => Ok(b),
        Ok(other) => Err(format!(
            "invalid config:\n{key}: expected boolean, got lua type {}",
            value_name(&other)
        )),
        Err(e) => Err(format!("invalid config:\n{key}: {e}")),
    }
}

fn harvest_titles(value: Value) -> Result<Titles, String> {
    let Value::Table(table) = value else {
        return Err(format!(
            "invalid config:\ntitles: expected table, got lua type {}",
            value_name(&value)
        ));
    };
    for pair in table.clone().pairs::<String, Value>() {
        let (key, val) = pair.map_err(|e| format!("invalid config:\ntitles: {e}"))?;
        if !TITLE_KEYS.contains(&key.as_str()) {
            return Err(format!("invalid config:\ntitles: unknown title '{key}'"));
        }
        if !matches!(val, Value::String(_)) {
            return Err(format!(
                "invalid config:\ntitles.{key}: expected string, got lua type {}",
                value_name(&val)
            ));
        }
    }
    Ok(Titles {
        filter: title_string(&table, "filter", "Filter")?,
        queue: title_string(&table, "queue", "Queue")?,
        mini: title_string(&table, "mini", "Mini Queue")?,
        info: title_string(&table, "info", "Info")?,
    })
}

fn title_string(table: &Table, key: &str, default: &str) -> Result<String, String> {
    match table.raw_get::<Value>(key) {
        Ok(Value::String(s)) => Ok(s.to_str().map_err(|e| e.to_string())?.to_string()),
        _ => Ok(default.to_string()),
    }
}

fn harvest_map(map: &Table) -> Result<std::collections::BTreeMap<String, Vec<String>>, String> {
    let mut out = std::collections::BTreeMap::new();
    for pair in map.clone().pairs::<String, Value>() {
        let (action, value) = pair.map_err(|e| format!("invalid config:\nvmp.map: {e}"))?;
        let keys = match value {
            Value::String(s) => vec![s.to_str().map_err(|e| e.to_string())?.to_string()],
            Value::Table(items) => {
                let mut list = Vec::new();
                for item in items.sequence_values::<Value>() {
                    let item =
                        item.map_err(|e| format!("invalid config:\nvmp.map.{action}: {e}"))?;
                    match item {
                        Value::String(s) => {
                            list.push(s.to_str().map_err(|e| e.to_string())?.to_string())
                        }
                        other => {
                            return Err(format!(
                                "invalid config:\nvmp.map.{action}: every key must be a string, got lua type {}",
                                value_name(&other)
                            ));
                        }
                    }
                }
                list
            }
            other => {
                return Err(format!(
                    "invalid config:\nvmp.map.{action}: expected key string or list, got lua type {}",
                    value_name(&other)
                ));
            }
        };
        out.insert(action, keys);
    }
    Ok(out)
}

fn value_name(value: &Value) -> &'static str {
    match value {
        Value::Nil => "nil",
        Value::Boolean(_) => "boolean",
        Value::Integer(_) | Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Table(_) => "table",
        Value::Function(_) | Value::Thread(_) => "function",
        _ => "userdata",
    }
}
