//! Read-only Docker Engine API collector.
//!
//! Only `GET` requests are ever issued. When the socket is missing or
//! unreadable the collector reports `available: false` and the rest of
//! scanscan keeps working.

use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

use scanscan_ipc::{ContainerInfo, DockerStats, ImageInfo, MountInfo, MountKind, VolumeInfo};

use crate::error::{CoreError, Result};

/// Availability of the Docker integration.
#[derive(Debug, Clone, Serialize)]
pub struct DockerStatus {
    pub available: bool,
    pub socket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Read-only collector bound to a Docker endpoint (`/path`, `unix://…` or `tcp://host:port`).
pub struct DockerCollector {
    target: String,
}

impl DockerCollector {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn status(&self) -> DockerStatus {
        match self.get("/version") {
            Ok(_) => DockerStatus {
                available: true,
                socket: self.target.clone(),
                error: None,
            },
            Err(e) => DockerStatus {
                available: false,
                socket: self.target.clone(),
                error: Some(e.to_string()),
            },
        }
    }

    /// Container list with sizes and mounts.
    pub fn containers(&self) -> Result<Vec<ContainerInfo>> {
        let value = self.get("/containers/json?size=1&all=1")?;
        let array = value
            .as_array()
            .ok_or_else(|| CoreError::Scan("unexpected docker response".into()))?;
        let mut out = Vec::with_capacity(array.len());
        for item in array {
            out.push(parse_container(item));
        }
        Ok(out)
    }

    /// Flattened mounts across all containers.
    pub fn mounts(&self) -> Result<Vec<MountInfo>> {
        let mut out = Vec::new();
        for container in self.containers()? {
            out.extend(container.mounts);
        }
        Ok(out)
    }

    /// One non-streaming stats sample per running container (fetched in parallel).
    pub fn stats(&self) -> Result<Vec<DockerStats>> {
        let value = self.get("/containers/json?all=0")?;
        let array = value.as_array().cloned().unwrap_or_default();
        let targets: Vec<(String, String)> = array
            .iter()
            .filter_map(|item| {
                let id = item.get("Id").and_then(Value::as_str)?.to_string();
                let name = item
                    .get("Names")
                    .and_then(Value::as_array)
                    .and_then(|names| names.first())
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim_start_matches('/')
                    .to_string();
                Some((id, name))
            })
            .collect();

        let out: Vec<DockerStats> = targets
            .par_iter()
            .filter_map(|(id, name)| {
                let sample = self
                    .get(&format!("/containers/{id}/stats?stream=false"))
                    .ok()?;
                Some(parse_stats(id, name, &sample))
            })
            .collect();
        Ok(out)
    }

    /// Image list with sizes and dangling detection.
    pub fn images(&self) -> Result<Vec<ImageInfo>> {
        let value = self.get("/images/json")?;
        let array = value.as_array().cloned().unwrap_or_default();
        Ok(array.iter().map(parse_image).collect())
    }

    /// Named volumes with usage data where available.
    pub fn volumes(&self) -> Result<Vec<VolumeInfo>> {
        let value = self.get("/volumes")?;
        let array = value
            .get("Volumes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(array.iter().map(parse_volume).collect())
    }

    fn get(&self, path: &str) -> Result<Value> {
        let body = http_get(&self.target, path)?;
        serde_json::from_slice(&body).map_err(CoreError::from)
    }
}

fn parse_container(item: &Value) -> ContainerInfo {
    let id = item.get("Id").and_then(Value::as_str).unwrap_or("").to_string();
    let name = item
        .get("Names")
        .and_then(Value::as_array)
        .and_then(|names| names.first())
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();
    let image = item.get("Image").and_then(Value::as_str).unwrap_or("").to_string();
    let state = item.get("State").and_then(Value::as_str).unwrap_or("").to_string();
    let size_rw = item.get("SizeRw").and_then(Value::as_u64);
    let size_root_fs = item.get("SizeRootFs").and_then(Value::as_u64);

    let mounts = item
        .get("Mounts")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|m| {
                    let kind = match m.get("Type").and_then(Value::as_str).unwrap_or("") {
                        "volume" => MountKind::Volume,
                        "tmpfs" => MountKind::Tmpfs,
                        "bind" => MountKind::Bind,
                        _ => MountKind::Bind,
                    };
                    MountInfo {
                        container_id: id.clone(),
                        container_name: name.clone(),
                        kind,
                        source: m.get("Source").and_then(Value::as_str).unwrap_or("").to_string(),
                        destination: m
                            .get("Destination")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        read_write: m.get("RW").and_then(Value::as_bool).unwrap_or(false),
                        node: None,
                        size: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    ContainerInfo {
        id,
        name,
        image,
        state,
        size_rw,
        size_root_fs,
        mounts,
    }
}

fn parse_image(item: &Value) -> ImageInfo {
    let repo_tags = item
        .get("RepoTags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    ImageInfo {
        id: item.get("Id").and_then(Value::as_str).unwrap_or("").to_string(),
        repo_tags,
        size: item.get("Size").and_then(Value::as_u64).unwrap_or(0),
        shared_size: item.get("SharedSize").and_then(Value::as_u64).unwrap_or(0),
        containers: item.get("Containers").and_then(Value::as_i64).unwrap_or(-1),
    }
}

fn parse_volume(item: &Value) -> VolumeInfo {
    let usage = item.get("UsageData");
    VolumeInfo {
        name: item.get("Name").and_then(Value::as_str).unwrap_or("").to_string(),
        driver: item.get("Driver").and_then(Value::as_str).unwrap_or("").to_string(),
        mountpoint: item
            .get("Mountpoint")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        size: usage.and_then(|u| u.get("Size")).and_then(Value::as_u64),
        ref_count: usage.and_then(|u| u.get("RefCount")).and_then(Value::as_i64),
    }
}

fn parse_stats(id: &str, name: &str, sample: &Value) -> DockerStats {
    let cpu = &sample["cpu_stats"];
    let pre = &sample["precpu_stats"];
    let cpu_total = cpu["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
    let pre_total = pre["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
    let sys = cpu["system_cpu_usage"].as_u64().unwrap_or(0);
    let pre_sys = pre["system_cpu_usage"].as_u64().unwrap_or(0);
    let online = cpu["online_cpus"].as_u64().unwrap_or(1).max(1);
    let cpu_delta = cpu_total.saturating_sub(pre_total) as f64;
    let sys_delta = sys.saturating_sub(pre_sys) as f64;
    let cpu_percent = if sys_delta > 0.0 {
        (cpu_delta / sys_delta) * online as f64 * 100.0
    } else {
        0.0
    };

    let mem_used = sample["memory_stats"]["usage"].as_u64().unwrap_or(0);
    let mem_limit = sample["memory_stats"]["limit"].as_u64().unwrap_or(0);

    let (mut net_rx, mut net_tx) = (0u64, 0u64);
    if let Some(networks) = sample["networks"].as_object() {
        for net in networks.values() {
            net_rx = net_rx.saturating_add(net["rx_bytes"].as_u64().unwrap_or(0));
            net_tx = net_tx.saturating_add(net["tx_bytes"].as_u64().unwrap_or(0));
        }
    }

    let (mut blk_read, mut blk_write) = (0u64, 0u64);
    if let Some(items) = sample["blkio_stats"]["io_service_bytes_recursive"].as_array() {
        for item in items {
            let op = item["op"].as_str().unwrap_or("");
            let value = item["value"].as_u64().unwrap_or(0);
            if op.eq_ignore_ascii_case("read") {
                blk_read = blk_read.saturating_add(value);
            } else if op.eq_ignore_ascii_case("write") {
                blk_write = blk_write.saturating_add(value);
            }
        }
    }

    DockerStats {
        id: id.to_string(),
        name: name.to_string(),
        cpu_percent,
        mem_used,
        mem_limit,
        net_rx,
        net_tx,
        blk_read,
        blk_write,
    }
}

fn http_get(target: &str, path: &str) -> Result<Vec<u8>> {
    if let Some(addr) = target
        .strip_prefix("tcp://")
        .or_else(|| target.strip_prefix("http://"))
    {
        let stream = std::net::TcpStream::connect(addr)?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        return http_request(stream, path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let path_only = target.strip_prefix("unix://").unwrap_or(target);
        let stream = UnixStream::connect(path_only)?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        http_request(stream, path)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(CoreError::Scan(
            "unix docker sockets are unsupported on this platform".into(),
        ))
    }
}

fn http_request<S: std::io::Read + std::io::Write>(mut stream: S, path: &str) -> Result<Vec<u8>> {
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: docker\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| CoreError::Scan("malformed docker HTTP response".into()))?;
    let headers = String::from_utf8_lossy(&buf[..split]).to_ascii_lowercase();
    let body = &buf[split + 4..];

    if headers.contains("transfer-encoding: chunked") {
        decode_chunked(body)
    } else {
        Ok(body.to_vec())
    }
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < body.len() {
        let line_end = body[i..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| CoreError::Scan("bad chunk header".into()))?;
        let size_str = std::str::from_utf8(&body[i..i + line_end])
            .map_err(|_| CoreError::Scan("bad chunk size".into()))?;
        let size = usize::from_str_radix(size_str.trim(), 16)
            .map_err(|_| CoreError::Scan("bad chunk size".into()))?;
        i += line_end + 2;
        if size == 0 {
            break;
        }
        if i + size > body.len() {
            return Err(CoreError::Scan("truncated chunk".into()));
        }
        out.extend_from_slice(&body[i..i + size]);
        i += size + 2;
    }
    Ok(out)
}
