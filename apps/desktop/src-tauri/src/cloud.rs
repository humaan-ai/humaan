//! RunPod provisioning and reconnectable jobs. Account credentials stay in process memory.
use crate::training::{id, list, load, save, timestamp, worker_source};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
#[cfg(not(test))]
use std::{
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const IMAGE: &str = "runpod/pytorch:2.8.0-py3.11-cuda12.8.1-cudnn-devel-ubuntu22.04";
const ENDPOINT: &str = "https://api.runpod.io/graphql";
fn credential() -> &'static Mutex<String> {
    static KEY: OnceLock<Mutex<String>> = OnceLock::new();
    KEY.get_or_init(|| Mutex::new(String::new()))
}
fn mutation_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
fn key() -> Result<String, String> {
    let s = credential()
        .lock()
        .map_err(|_| "Credential lock unavailable")?
        .clone();
    if s.is_empty() {
        Err(
            "Connect your RunPod account first. API keys are kept only for this app session."
                .into(),
        )
    } else {
        Ok(s)
    }
}
fn valid_token(s: &str) -> bool {
    (16..=512).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}
fn identifier(s: &str) -> Result<&str, String> {
    if s.is_empty() || s.len() > 100 || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
        Err("Invalid provider resource ID".into())
    } else {
        Ok(s)
    }
}
#[cfg(test)]
thread_local! {static RESPONSES:std::cell::RefCell<Vec<Result<Value,String>>>=const{std::cell::RefCell::new(Vec::new())};static REQUESTS:std::cell::RefCell<Vec<Value>>=const{std::cell::RefCell::new(Vec::new())};}
fn request(url: &str, token: &str, body: Option<Value>) -> Result<Value, String> {
    #[cfg(test)]
    {
        let _ = token;
        REQUESTS.with(|r| r.borrow_mut().push(json!({"url":url,"body":body})));
        return RESPONSES.with(|r| {
            let mut q = r.borrow_mut();
            if q.is_empty() {
                panic!("Unexpected HTTP request {url}")
            }
            q.remove(0)
        });
    }
    #[cfg(not(test))]
    http_request(url, token, body)
}
#[cfg(not(test))]
fn http_request(url: &str, token: &str, body: Option<Value>) -> Result<Value, String> {
    if !valid_token(token) {
        return Err("Invalid authentication token.".into());
    }
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    // Secrets use a private temporary config file, never process arguments or project metadata.
    let config = dir.path().join("curl.conf");
    let output = dir.path().join("output");
    let error = dir.path().join("error");
    fs::write(&config,format!("header = \"Authorization: Bearer {token}\"\nheader = \"Content-Type: application/json\"\n")).map_err(|e|e.to_string())?;
    let mut command = Command::new("curl");
    command.args([
        "--disable",
        "--silent",
        "--show-error",
        "--fail",
        "--proto",
        "=https",
        "--connect-timeout",
        "15",
        "--max-time",
        "120",
        "--max-filesize",
        "33554432",
        "--config",
        config.to_str().ok_or("Invalid temp path")?,
        "--output",
        output.to_str().ok_or("Invalid temp path")?,
    ]);
    if let Some(body) = body {
        let path = dir.path().join("body.json");
        let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
        if bytes.len() > 80 * 1024 * 1024 {
            return Err("Experiment package exceeds 80 MB.".into());
        }
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        command.args(["--data-binary", &format!("@{}", path.display())]);
    }
    let mut child = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(fs::File::create(&error).map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("HTTPS request could not start (curl is required): {e}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(125)
            || fs::metadata(&output)
                .map(|m| m.len() > 32 * 1024 * 1024)
                .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Remote request exceeded its time or size limit. Refresh before retrying.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        return Err(fs::read_to_string(error)
            .unwrap_or_default()
            .replace(token, "[redacted]")
            .chars()
            .take(1000)
            .collect());
    }
    serde_json::from_slice(&fs::read(output).map_err(|e| e.to_string())?)
        .map_err(|_| "Invalid JSON from remote service".into())
}
fn graphql(token: &str, query: &str, variables: Value) -> Result<Value, String> {
    let response = request(
        ENDPOINT,
        token,
        Some(json!({"query":query,"variables":variables})),
    )?;
    if let Some(errors) = response["errors"].as_array() {
        if !errors.is_empty() {
            return Err(format!(
                "RunPod: {}",
                errors[0]["message"].as_str().unwrap_or("Request failed")
            )
            .replace(token, "[redacted]"));
        }
    }
    if !response["data"].is_object() {
        return Err("RunPod returned no data.".into());
    }
    Ok(response["data"].clone())
}
fn identity(token: &str) -> Result<String, String> {
    graphql(token, "query { myself { id } }", json!({}))?["myself"]["id"]
        .as_str()
        .map(String::from)
        .ok_or("RunPod account unavailable".into())
}
fn connect(token: String) -> Result<Value, String> {
    if !valid_token(&token) {
        return Err("Enter a valid RunPod API key.".into());
    }
    let data = graphql(
        &token,
        "query { myself { id } gpuTypes { id displayName memoryInGb secureCloud } }",
        json!({}),
    )?;
    if !data["myself"]["id"].is_string() {
        return Err("RunPod account unavailable".into());
    }
    *credential().lock().unwrap() = token;
    Ok(json!({"accountId":data["myself"]["id"],"gpus":data["gpuTypes"]}))
}
fn validate(config: &Value) -> Result<(), String> {
    if config["gpuTypeId"]
        .as_str()
        .is_none_or(|s| s.is_empty() || s.len() > 150)
    {
        return Err("Select a GPU.".into());
    }
    let country = config["countryCode"].as_str().ok_or("Select a region")?;
    if !country.is_empty()
        && !(country.len() == 2 && country.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return Err("Invalid region".into());
    }
    if !config["minutes"]
        .as_u64()
        .is_some_and(|m| (10..=120).contains(&m))
    {
        return Err("Runtime must be 10–120 minutes.".into());
    }
    if !["stop", "terminate"].contains(&config["shutdown"].as_str().unwrap_or_default()) {
        return Err("Invalid shutdown behavior".into());
    }
    Ok(())
}
fn quote(root: &str, config: Value, experiment_id: &str) -> Result<Value, String> {
    validate(&config)?;
    let token = key()?;
    let account = identity(&token)?;
    let experiment = load(root, "experiments", experiment_id)?;
    let mut filters =
        json!({"gpuCount":1,"secureCloud":true,"minMemoryInGb":16,"minVcpuCount":4,"totalDisk":40});
    if config["countryCode"] != "" {
        filters["countryCode"] = config["countryCode"].clone();
    }
    let prices=graphql(&token,"query($gpu: GpuTypeFilter, $filter: GpuLowestPriceInput!) { gpuTypes(input: $gpu) { id displayName memoryInGb lowestPrice(input: $filter) { uninterruptablePrice stockStatus } } }",json!({"gpu":{"id":config["gpuTypeId"]},"filter":filters}))?;
    let gpu = &prices["gpuTypes"][0];
    let rate = gpu["lowestPrice"]["uninterruptablePrice"]
        .as_f64()
        .filter(|p| p.is_finite() && *p > 0.)
        .ok_or(
            "No current on-demand quote for this GPU and region. Choose another configuration.",
        )?;
    let q = json!({"id":id(),"createdAt":timestamp(),"expiresAt":timestamp()+10*60*1000,"config":config,"accountId":account,"experimentId":experiment_id,"experimentName":experiment["name"],"gpuName":gpu["displayName"],"hourlyUsd":rate,"estimatedComputeUsd":rate*config["minutes"].as_f64().unwrap()/60.,"imageName":IMAGE});
    save(root, "cloud-quotes", &q, false)?;
    Ok(q)
}
fn verify(job: &Value, token: &str) -> Result<(), String> {
    if identity(token)? != job["accountId"].as_str().ok_or("Missing account")? {
        return Err("Connect the RunPod account that owns this job.".into());
    }
    Ok(())
}
fn public_job(mut job: Value) -> Value {
    for k in ["experiment", "workerToken"] {
        job.as_object_mut().unwrap().remove(k);
    }
    job
}
fn base64(data: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0] {
            s.push(A[((n >> shift) & 63) as usize] as char);
        }
        if c.len() < 3 {
            s.pop();
            s.push('=');
        }
        if c.len() == 1 {
            let i = s.len() - 2;
            s.replace_range(i..i + 1, "=");
        }
    }
    s
}
fn create_input(job: &Value) -> Result<Value, String> {
    let c = &job["config"];
    let deadline = chrono::DateTime::from_timestamp_millis(
        (timestamp() + c["minutes"].as_u64().unwrap() * 60000) as i64,
    )
    .ok_or("Invalid deadline")?
    .to_rfc3339();
    let encoded = base64(include_str!("cloud_worker.py").as_bytes());
    let command = format!("python3 -u -c \"import base64;exec(base64.b64decode('{encoded}'))\"");
    let mut input = json!({"name":format!("humaan-{}",job["id"].as_str().unwrap()),"cloudType":"SECURE","gpuCount":1,"gpuTypeId":c["gpuTypeId"],"imageName":IMAGE,"dockerArgs":command,"ports":"8080/http","containerDiskInGb":20,"volumeInGb":20,"volumeMountPath":"/workspace","minVcpuCount":4,"minMemoryInGb":16,"startSsh":false,"startJupyter":false,"env":[{"key":"HUMAAN_JOB_TOKEN","value":job["workerToken"]}]});
    if c["countryCode"] != "" {
        input["countryCode"] = c["countryCode"].clone();
    }
    input[if c["shutdown"] == "stop" {
        "stopAfter"
    } else {
        "terminateAfter"
    }] = json!(deadline);
    Ok(input)
}
fn launch(root: &str, quote_id: &str) -> Result<Value, String> {
    let q = load(root, "cloud-quotes", quote_id)?;
    validate(&q["config"])?;
    if q["expiresAt"].as_u64().unwrap_or(0) < timestamp() {
        return Err("Estimate expired. Review a new estimate.".into());
    }
    let token = key()?;
    verify(&q, &token)?;
    let experiment = load(
        root,
        "experiments",
        q["experimentId"].as_str().ok_or("Missing experiment")?,
    )?;
    if serde_json::to_vec(&json!({"experiment":experiment,"worker":worker_source()}))
        .map_err(|e| e.to_string())?
        .len()
        > 80 * 1024 * 1024
    {
        return Err("Experiment package exceeds the 80 MB remote upload limit.".into());
    }
    let mut job = json!({"id":quote_id,"createdAt":timestamp(),"name":experiment["name"],"experimentId":experiment["id"],"config":q["config"],"accountId":q["accountId"],"quote":q,"status":"provisioning","instanceState":"unknown","experiment":experiment,"workerToken":format!("{}{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple())});
    // No automatic retry of a create mutation. Persist first and recover by unique job name.
    save(root, "cloud-jobs", &job, false)?;
    match graphql(&token,"mutation($input: PodFindAndDeployOnDemandInput) { podFindAndDeployOnDemand(input: $input) { id desiredStatus costPerHr } }",json!({"input":create_input(&job)?})){Ok(data)=>{let pod=&data["podFindAndDeployOnDemand"];if pod["id"].is_string(){job["instanceId"]=pod["id"].clone();job["instanceState"]=pod["desiredStatus"].clone();job["actualHourlyUsd"]=pod["costPerHr"].clone();}else{job["status"]=json!("launch-uncertain");job["error"]=json!("No pod ID returned. Refresh to discover it before creating another job.");}},Err(e)=>{job["status"]=json!("launch-uncertain");job["error"]=json!(e);}}
    save(root, "cloud-jobs", &job, true)?;
    Ok(public_job(job))
}
fn pod(job: &Value, token: &str) -> Result<Value, String> {
    let data=graphql(token,"query { myself { id pods { id name desiredStatus costPerHr runtime { gpus { gpuUtilPercent memoryUtilPercent } } } } }",json!({}))?;
    if data["myself"]["id"] != job["accountId"] {
        return Err("Wrong RunPod account.".into());
    }
    let name = format!("humaan-{}", job["id"].as_str().ok_or("Missing job ID")?);
    let matches: Vec<Value> = data["myself"]["pods"]
        .as_array()
        .ok_or("RunPod returned no pod list")?
        .iter()
        .filter(|p| {
            if job["instanceId"].is_null() {
                p["name"] == name
            } else {
                p["id"] == job["instanceId"]
            }
        })
        .cloned()
        .collect();
    if matches.len() > 1 {
        return Err(
            "Multiple pods match this job. Inspect them in RunPod before continuing.".into(),
        );
    }
    Ok(matches.first().cloned().unwrap_or(Value::Null))
}
fn remote(job: &Value, path: &str, body: Option<Value>) -> Result<Value, String> {
    let pod_id = identifier(job["instanceId"].as_str().ok_or("Pod is not ready")?)?;
    request(
        &format!("https://{pod_id}-8080.proxy.runpod.net/{path}"),
        job["workerToken"].as_str().ok_or("Missing job token")?,
        body,
    )
}
fn refresh(root: &str, job_id: &str) -> Result<Value, String> {
    let mut job = load(root, "cloud-jobs", job_id)?;
    let token = key()?;
    let vm = pod(&job, &token)?;
    if vm.is_null() {
        if job["instanceId"].is_string() {
            job["instanceState"] = json!("TERMINATED");
        }
        save(root, "cloud-jobs", &job, true)?;
        return Ok(public_job(job));
    }
    job["instanceId"] = vm["id"].clone();
    job["instanceState"] = vm["desiredStatus"].clone();
    job["actualHourlyUsd"] = vm["costPerHr"].clone();
    job["gpuMetrics"] = vm["runtime"]["gpus"].clone();
    job["lastCheckedAt"] = json!(timestamp());
    if job["instanceState"] == "RUNNING" {
        match remote(&job, "status", None) {
            Ok(mut state) => {
                job["error"] = Value::Null;
                if state["status"] == "waiting"
                    && (job["submitted"] == true || job["cancelRequested"] == true)
                {
                    state["status"] = json!("interrupted");
                    state["error"] = json!("Previously submitted workload is missing from the pod volume. Create a new job to retry.");
                } else if state["status"] == "waiting" {
                    let package = json!({"experiment":job["experiment"],"worker":worker_source()});
                    match remote(&job, "start", Some(package)) {
                        Ok(_) => {
                            state["status"] = json!("preparing");
                            job["submitted"] = json!(true);
                        }
                        Err(e) => job["error"] = json!(e),
                    }
                }
                let status = state["status"].as_str().ok_or("Invalid worker status")?;
                if ![
                    "waiting",
                    "preparing",
                    "running",
                    "completed",
                    "failed",
                    "interrupted",
                ]
                .contains(&status)
                {
                    return Err("Invalid worker status".into());
                }
                job["status"] = json!(status);
                job["logs"] = state["logs"].clone();
                job["remote"] = state;
            }
            Err(e) => job["error"] = json!(format!("Worker not reachable yet: {e}")),
        }
    }
    save(root, "cloud-jobs", &job, true)?;
    Ok(public_job(job))
}
fn retrieve(root: &str, job_id: &str) -> Result<Value, String> {
    let mut job = load(root, "cloud-jobs", job_id)?;
    let token = key()?;
    verify(&job, &token)?;
    let result = remote(&job, "result", None)?;
    if result["experimentId"] != job["experimentId"]
        || !result["policy"].is_object()
        || !result["evaluation"].is_object()
        || !result["baseline"].is_object()
        || !result["frames"].is_array()
    {
        return Err("Remote artifact is not a complete result for this experiment.".into());
    }
    let e = &job["experiment"];
    let run = json!({"id":job_id,"createdAt":job["createdAt"],"finishedAt":timestamp(),"name":job["name"],"status":"completed","experimentId":e["id"],"experiment":e,"versionId":e["versionId"],"robotName":e["robot"]["name"],"experienceId":e["experience"]["id"],"loopId":e["loop"]["id"],"execution":{"kind":"runpod","jobId":job_id,"instanceId":job["instanceId"]},"result":result});
    match load(root, "runs", job_id) {
        Ok(old) if old["execution"]["jobId"] == job_id => {}
        Ok(_) => return Err("Run ID conflict".into()),
        Err(_) => save(root, "runs", &run, false)?,
    }
    job["retrieved"] = json!(true);
    save(root, "cloud-jobs", &job, true)?;
    Ok(public_job(job))
}
fn control(root: &str, job_id: &str, action: &str) -> Result<Value, String> {
    let mut job = load(root, "cloud-jobs", job_id)?;
    let token = key()?;
    let vm = pod(&job, &token)?;
    if vm.is_null() {
        return Err("Pod not found in the connected account. Refresh before retrying.".into());
    }
    let query = match action {
        "stop" => "mutation($input: PodStopInput!) { podStop(input: $input) { id desiredStatus } }",
        "release" => "mutation($input: PodTerminateInput!) { podTerminate(input: $input) }",
        _ => return Err("Unknown control action".into()),
    };
    graphql(&token, query, json!({"input":{"podId":vm["id"]}}))?;
    job["cancelRequested"] = json!(true);
    job["instanceId"] = vm["id"].clone();
    job["instanceState"] = json!(if action == "stop" {
        "STOPPING"
    } else {
        "TERMINATING"
    });
    save(root, "cloud-jobs", &job, true)?;
    Ok(public_job(job))
}
#[tauri::command]
pub async fn cloud_connect(api_key: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || connect(api_key))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cloud_jobs(root: String) -> Result<Vec<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(list(&root, "cloud-jobs")?
            .into_iter()
            .map(public_job)
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cloud_quote(
    root: String,
    config: Value,
    experiment_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || quote(&root, config, &experiment_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cloud_launch(root: String, quote_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutation_lock()
            .lock()
            .map_err(|_| "Cloud lock unavailable")?;
        launch(&root, &quote_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cloud_job_action(root: String, id: String, action: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = mutation_lock()
            .lock()
            .map_err(|_| "Cloud lock unavailable")?;
        match action.as_str() {
            "refresh" => refresh(&root, &id),
            "retrieve" => retrieve(&root, &id),
            "stop" | "release" => control(&root, &id, &action),
            _ => Err("Unknown job action".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn responses(values: Vec<Value>) {
        RESPONSES.with(|r| *r.borrow_mut() = values.into_iter().map(Ok).collect());
    }
    fn account() -> Value {
        json!({"data":{"myself":{"id":"account-test"}}})
    }
    fn pod_list(name: &str) -> Value {
        json!({"data":{"myself":{"id":"account-test","pods":[{"id":"pod123","name":name,"desiredStatus":"RUNNING","costPerHr":0.5,"runtime":{"gpus":[]}}]}}})
    }
    #[test]
    fn validates_and_keeps_secrets_out_of_public_jobs() {
        assert!(!valid_token("secret\nheader"));
        assert!(valid_token("rpa_abcdefghijklmnopqrstuv"));
        assert!(identifier("pod/../../bad").is_err());
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        let public = public_job(json!({"id":"1","experiment":{},"workerToken":"secret"}));
        assert!(public["workerToken"].is_null());
        assert!(public["experiment"].is_null());
    }
    #[test]
    fn lifecycle_preserves_snapshot_recovers_unknown_launch_and_retrieves_results() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Cloud").unwrap();
        let root = dir.path().to_str().unwrap();
        let experiment = crate::training::tests::fixture(root);
        *credential().lock().unwrap() = "rpa_testtesttesttesttest".into();
        let c = json!({"gpuTypeId":"NVIDIA RTX A6000","countryCode":"US","shutdown":"stop","minutes":30});
        responses(vec![
            account(),
            json!({"data":{"gpuTypes":[{"id":"NVIDIA RTX A6000","displayName":"A6000","lowestPrice":{"uninterruptablePrice":0.5}}]}}),
        ]);
        let q = quote(root, c, experiment["id"].as_str().unwrap()).unwrap();
        assert_eq!(q["estimatedComputeUsd"], 0.25);
        let id = q["id"].as_str().unwrap();
        let name = format!("humaan-{id}");
        RESPONSES.with(|r| {
            *r.borrow_mut() = vec![
                Ok(account()),
                Err("Connection lost after accepting launch".into()),
            ]
        });
        let job = launch(root, id).unwrap();
        assert_eq!(job["status"], "launch-uncertain");
        responses(vec![account()]);
        assert!(launch(root, id).is_err());
        let private = load(root, "cloud-jobs", id).unwrap();
        let input = create_input(&private).unwrap();
        assert!(input["stopAfter"].is_string());
        assert!(input["terminateAfter"].is_null());
        assert_eq!(input["volumeInGb"], 20);
        assert!(!input.to_string().contains("rpa_testtest"));
        responses(vec![
            pod_list(&name),
            json!({"status":"waiting"}),
            json!({"status":"preparing"}),
        ]);
        let prepared = refresh(root, id).unwrap();
        assert_eq!(prepared["submitted"], true);
        assert_eq!(prepared["instanceId"], "pod123");
        let uploaded = REQUESTS.with(|r| r.borrow().last().unwrap()["body"].clone());
        assert_eq!(uploaded["experiment"], experiment);
        assert!(uploaded["worker"]
            .as_str()
            .unwrap()
            .contains("HUMAAN_PROGRESS"));
        responses(vec![
            pod_list("renamed-in-console"),
            json!({"status":"completed","logs":"complete"}),
        ]);
        assert_eq!(refresh(root, id).unwrap()["status"], "completed");
        let result = json!({"experimentId":experiment["id"],"policy":{},"evaluation":{},"baseline":{},"frames":[]});
        responses(vec![account(), result]);
        assert_eq!(retrieve(root, id).unwrap()["retrieved"], true);
        assert_eq!(load(root, "runs", id).unwrap()["experiment"], experiment);
        responses(vec![pod_list(&name), json!({"data":{"podTerminate":null}})]);
        assert_eq!(
            control(root, id, "release").unwrap()["instanceState"],
            "TERMINATING"
        );
        responses(vec![
            json!({"data":{"myself":{"id":"account-test","pods":[]}}}),
        ]);
        assert_eq!(refresh(root, id).unwrap()["instanceState"], "TERMINATED");
        responses(vec![json!({"data":{"myself":{"id":"different-account"}}})]);
        assert!(retrieve(root, id).unwrap_err().contains("owns this job"));
        let requests = REQUESTS.with(|r| r.borrow().clone());
        assert_eq!(
            requests
                .iter()
                .filter(|r| r["body"]["query"]
                    .as_str()
                    .unwrap_or("")
                    .contains("mutation($input: PodFindAndDeploy"))
                .count(),
            1
        );
    }
}
