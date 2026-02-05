# kcap

**1. 背景与目标**

- 目标用户：运维、研发，需要在生产或测试 K8S 集群中快速抓包分析端口流量。
- 目标能力：
  - 通过 SSH 连接到集群节点，或通过 Pod 直接抓包。
  - 支持选择命名空间、Pod、容器、端口、协议。
  - 在远端抓包并输出 pcap/pcapng（Wireshark 可直接打开）。
  - 支持本地保存与实时流式导出。
- 非目标：
  - 不做复杂 UI（仅 CLI）。
  - 不内置 K8S API 权限管理（由用户的 kubeconfig 或 SSH 权限控制）。

**2. 关键使用场景**

- SRE 需要对 `namespace=prod` 中某个服务 443 端口异常流量做分析。
- 研发需要对某个 Pod 内部抓包，定位应用层错误。
- 需要通过跳板机（SSH）进入集群节点进行抓包。

**3. 设计约束**

- 远端必须具备抓包能力（tcpdump 或 tshark）。
- Rust 实现，优先使用稳定库：
  - SSH：`ssh2` 或 `russh`。
  - CLI：`clap`。
  - 日志：`tracing`。
- Wireshark 格式：优先 `pcapng`，兼容 `pcap`。
- 跨平台：至少支持 Linux/macOS，Windows 作为可选目标。

**4. 总体架构**

```
CLI
 ├── Config Loader
 ├── SSH Client
 │   ├── Jump Host Support (ProxyCommand)
 │   └── Remote Exec (tcpdump/tshark)
 ├── Remote Capture
 │   ├── Port Filter Generator
 │   └── Packet Stream
 └── Local Output
     ├── File Writer (pcap/pcapng)
     └── Stream (stdout/pipe)
```

**5. 核心流程**

1. 用户指定目标（节点或 Pod）。
2. 工具通过 SSH 连接到节点，或通过 `kubectl exec` 进入 Pod（必要时使用 jump host）。
3. 在远端执行 `tcpdump -i <iface> <filter> -w -`（写到 stdout）。
4. 本地接收二进制流并保存为 `.pcap` 或 `.pcapng`。
5. 用户用 Wireshark 打开分析。

**6. 模块设计**

**6.1 CLI 模块**

- 命令：`kcap`
- 关键参数：
  - `--ssh-user`
  - `--ssh-host`
  - `--ssh-port`
  - `--jump-host`（可选）
  - `--namespace`
  - `--pod`
  - `--container`
  - `--port`
  - `--protocol` (`tcp|udp|all`)
  - `--iface`（默认 `any`）
  - `--output`（默认 `capture.pcap`）
  - `--format` (`pcap|pcapng`)
  - `--duration`（可选）
  - `--filter`（自定义 tcpdump 表达式）

**6.2 K8S 目标解析模块**

- 目标解析策略：
  - 若 `--ssh-host` 指定节点，直接抓。
  - 若指定 Pod：
    - 通过 `kubectl exec` 直接在 Pod 内抓包（需要容器权限）。
- 依赖：调用 `kubectl`（外部命令），避免引入 k8s API 客户端复杂度。

**6.3 SSH 模块**

- 支持：
  - 密码/私钥认证。
  - jump host（通过 `ProxyCommand` 或二次 SSH 链接）。
- 远端执行：
  - `tcpdump -i any port 443 -w -`
  - 捕获 stdout 的二进制流。

**6.4 抓包模块**

- 过滤规则生成：
  - `port 443`
  - `tcp port 443`
  - `udp port 53`
- 支持附加 filter：`(port 443) and host 10.0.0.5`
- 远端执行 tcpdump 时加 `-U`（实时刷新）和 `-s 0`（抓全包）。

**6.5 输出模块**

- 默认写本地文件：
  - `capture-YYYYMMDD-HHMMSS.pcap`
- 支持 `stdout`（用于管道传输，比如直接喂给 Wireshark）。

**7. 安全与权限**

- SSH 需要足够权限执行 `tcpdump`。
- 对 Pod 内抓包，可能需要 `NET_ADMIN` 或特权容器。
- 不在工具中缓存敏感凭证。

**8. 错误处理**

- 常见错误：
  - SSH 连接失败。
  - `tcpdump` 未安装。
  - 权限不足（`tcpdump: You don't have permission...`）。
- 处理策略：
  - 明确错误消息（stderr 输出）。
  - 提示安装或提权建议。

**9. 关键技术选型**

- SSH：`ssh2`（libssh2 绑定）或 `russh`。
- CLI：`clap`。
- 进程执行：`std::process::Command`。
- 日志：`tracing`。

**10. 里程碑**

1. MVP：支持 SSH 连接节点 + tcpdump 抓包 + pcap 输出。
2. 增强：支持 Pod 定位 + jump host。
3. 高级：支持实时输出 Wireshark（通过管道或 WebSocket）。

**11. 示例使用**

```
kcap --ssh-user root --ssh-host 10.0.0.10 --port 443 --protocol tcp --output https.pcap

kcap --namespace prod --pod orders-6c9f --port 8080 --output orders.pcap

kcap --ssh-host 10.0.0.10 --port 53 --protocol udp --format pcapng --duration 60
```

**12. 风险与替代方案**

- 远端没有 tcpdump：提示安装或使用 `tshark`。
- Pod 内抓包权限不足：提示使用特权容器或直接在节点抓。
- 高流量抓包导致 IO 高：建议加 filter 或限制 duration。

**13. Pod 容器内抓包**

- 提供 `--pod` 时，工具通过 `kubectl exec` 在 Pod 内抓包，而不是 SSH 到节点。
- `--container` 映射为 `kubectl exec -c <container>`。
- 容器需要有抓包权限（`NET_ADMIN`）或使用特权容器。

**14. Pod 抓包示例**

```
kcap --namespace prod --pod orders-6c9f --container api --port 8080 --output orders.pcap
```
