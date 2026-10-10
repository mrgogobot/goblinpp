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

fn goblin_data_finish(loop_steps: u64) -> Result<(), String> {
    GOBLIN_DATA.with(|data| {
        let data = data.borrow();
        data.batches.finish().map_err(|e|e.pretty())?;
        let imports = data.data_imports();
        let manifest = if let Ok(path) = std::env::var("GOBLIN_NATIVE_DATA_PATH") {
            std::path::PathBuf::from(path)
        } else if imports.is_empty() && data.generated.is_empty() && data.batches.evidence()["writers"].as_array().is_some_and(|w|w.is_empty()) && data.randomness.evidence().streams.is_empty() && data.inference.evidence()["functions"].as_array().is_some_and(|f| f.is_empty()) {
            return Ok(());
        } else {
            let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
            let dir = std::path::PathBuf::from(format!("goblin-native-outputs-{}-{stamp}", std::process::id()));
            std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
            eprintln!("NATIVE_DATA_DIR={}", dir.display());
            dir.join("native-data.json")
        };
        let directory = manifest.parent().ok_or("Native data manifest has no directory.")?.join("native-outputs");
        if std::env::var("GOBLIN_NATIVE_DATA_PATH").is_err() {
            preserve_batch_inputs(&data,manifest.parent().unwrap())?;
        }
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let mut artifacts = Vec::new();
        for output in data.generated.values() {
            let path = directory.join(&output.name);
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
            file.write_all(&output.bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            artifacts.push(goblinpp::output::artifact_descriptor(output));
        }
        artifacts.extend(data.batches.publish(&directory,true).map_err(|e|e.pretty())?);
        artifacts.sort_by(|a,b|a["name"].as_str().cmp(&b["name"].as_str()));
        let value = serde_json::json!({"schema":"goblin.native-data.v1", "goblin_version":goblinpp::VERSION, "data_imports":imports, "generated_artifacts":artifacts, "rng_policy":goblinpp::random::policy(), "rng":data.randomness.evidence(), "inference_policy":goblinpp::inference_policy::policy(), "inference":data.inference.evidence(), "resource_policy":goblinpp::resources::policy(), "resources":goblinpp::resources::evidence(REQUESTED_LOOP_BUDGET,loop_steps), "batch_policy":goblinpp::batches::policy(),"batches":data.batches.evidence()});
        let mut file = OpenOptions::new().write(true).create_new(true).open(&manifest).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn goblin_data_abort() {
    GOBLIN_DATA.with(|data| {
        let mut data=data.borrow_mut();
        if data.batches.evidence()["writers"].as_array().is_some_and(|w|w.is_empty())
            && data.batches.evidence()["readers"].as_array().is_some_and(|r|r.is_empty()) {
                data.batches=goblinpp::batches::Batches::default();
                return;
            }
        let saved=(||->Result<(),String>{
            let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
            let directory=std::path::PathBuf::from(format!("goblin-native-incomplete-{}-{stamp}",std::process::id()));
            std::fs::create_dir(&directory).map_err(|e|e.to_string())?;
            preserve_batch_inputs(&data,&directory)?;
            let artifacts=data.batches.publish(&directory.join("outputs"),false).map_err(|e|e.pretty())?;
            let evidence=serde_json::json!({"schema":"goblin.native-batch-failure.v1","status":"INCOMPLETE","batch_policy":goblinpp::batches::policy(),"batches":data.batches.evidence(),"generated_artifacts":artifacts});
            let mut file=OpenOptions::new().write(true).create_new(true).open(directory.join("batch-failure.json")).map_err(|e|e.to_string())?;
            file.write_all(&serde_json::to_vec_pretty(&evidence).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            file.sync_all().map_err(|e|e.to_string())?;
            eprintln!("INCOMPLETE_DATA_DIR={}",directory.display());
            Ok(())
        })();
        if let Err(error)=saved {
            eprintln!("Unable to preserve incomplete stream outputs: {error}");
            if let Some(directory)=data.batches.staging_directory() {eprintln!("RECOVERABLE_STAGING_DIR={}",directory.display());}
        } else {
            // process::exit does not run TLS destructors: clean up only after
            // successfully preserving the disk snapshots and partial outputs.
            data.batches=goblinpp::batches::Batches::default();
        }
    });
}

fn preserve_batch_inputs(data: &goblinpp::evaluator::Evaluation, directory: &std::path::Path) -> Result<(),String> {
    if data.batches.imports().is_empty() {return Ok(());}
    let mut inputs=Vec::new();
    let inputs_dir=directory.join("batch-inputs");
    std::fs::create_dir(&inputs_dir).map_err(|e|e.to_string())?;
    for import in data.batches.imports() {
        let path=inputs_dir.join(&import.sha256);
        if !path.exists() {
            data.batches.copy_input(&import.sha256,&path).ok_or("Missing batch snapshot")?.map_err(|e|e.pretty())?;
        }
        inputs.push(serde_json::json!({"source":import.path,"path":format!("batch-inputs/{}",import.sha256),"sha256":import.sha256,"byte_count":import.byte_count}));
    }
    let mut file=OpenOptions::new().write(true).create_new(true).open(directory.join("batch-inputs.json")).map_err(|e|e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&inputs).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    file.sync_all().map_err(|e|e.to_string())?;
    Ok(())
}
