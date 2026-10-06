// Included only in programs using native scientific I/O. This links shared Rust
// helpers at build time; it never invokes Goblin++ or parses .gbl source at runtime.
thread_local! {
    static GOBLIN_DATA: std::cell::RefCell<goblinpp::evaluator::Evaluation> =
        std::cell::RefCell::new({
            let mut evaluation = goblinpp::evaluator::Evaluation::new(
                std::env::var("GOBLIN_NATIVE_DATA_BASE").unwrap_or_else(|_| ".".into()));
            let mut argv = vec![std::env::var("GOBLIN_SOURCE_ARGV0").unwrap_or_else(|_| std::env::args().next().unwrap_or_default())];
            argv.extend(std::env::args().skip(1));
            evaluation.configure_interaction(argv, goblinpp::interaction::InputPolicy::Disabled).expect("valid native argv");
            evaluation
        });
}

fn data_value(value: Value) -> Result<goblinpp::evaluator::Value, String> {
    use goblinpp::evaluator::Value as V;
    Ok(match value {
        Value::Q(v, d) => V::Quantity(goblinpp::quantity::Quantity::new(v, d).map_err(|e| e.pretty())?),
        Value::Text(s) => V::Text(s),
        Value::Bool(b) => V::Bool(b),
        Value::Array(a) => V::Array(a.into_iter().map(data_value).collect::<Result<_,_>>()?),
    })
}

fn from_data_value(value: goblinpp::evaluator::Value) -> Value {
    use goblinpp::evaluator::Value as V;
    match value {
        V::Quantity(q) => Value::Q(q.value_si, q.dimension),
        V::Text(s) => Value::Text(s),
        V::Bool(b) => Value::Bool(b),
        V::Array(a) => Value::Array(a.into_iter().map(from_data_value).collect()),
    }
}

fn goblin_data_call(name: &str, values: Vec<Result<Value, String>>, env: &HashMap<String, Value>) -> Result<Value, String> {
    let values = values.into_iter().map(|v| data_value(v?)).collect::<Result<Vec<_>,_>>()?;
    let env = env.iter().map(|(k,v)| Ok((k.clone(), data_value(v.clone())?))).collect::<Result<HashMap<_,_>,String>>()?;
    GOBLIN_DATA.with(|data| data.borrow_mut().native_data_call(name, values, env).map(from_data_value).map_err(|e| e.pretty()))
}

fn goblin_data_finish() -> Result<(), String> {
    GOBLIN_DATA.with(|data| {
        let data = data.borrow();
        let imports = data.data_imports();
        let manifest = if let Ok(path) = std::env::var("GOBLIN_NATIVE_DATA_PATH") {
            std::path::PathBuf::from(path)
        } else if imports.is_empty() && data.generated.is_empty() && data.randomness.evidence().streams.is_empty() {
            return Ok(());
        } else {
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
            let dir = std::path::PathBuf::from(format!("goblin-native-outputs-{}-{stamp}", std::process::id()));
            std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
            eprintln!("NATIVE_DATA_DIR={}", dir.display());
            dir.join("native-data.json")
        };
        let directory = manifest.parent().ok_or("Native data manifest has no directory.")?.join("native-outputs");
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let mut artifacts = Vec::new();
        for output in data.generated.values() {
            let path = directory.join(&output.name);
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
            file.write_all(&output.bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            artifacts.push(goblinpp::output::artifact_descriptor(output));
        }
        let value = serde_json::json!({"schema":"goblin.native-data.v1", "data_imports":imports, "generated_artifacts":artifacts, "rng":data.randomness.evidence()});
        let mut file = OpenOptions::new().write(true).create_new(true).open(&manifest).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })
}
