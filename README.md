# AwsomeFS – A Cluster-Aware Distributed Filesystem for Kubernetes

**AwsomeFS** is an experimental, shared-block distributed filesystem designed for Kubernetes clusters. It combines shared device coordination, metadata-driven consistency, and CSI integration to offer a developer-friendly yet enterprise-aware platform for building and running stateful workloads with fine-grained control over storage behavior.

---

## ⚡️ Why AwsomeFS?

Traditional distributed filesystems like Ceph, GlusterFS, and Longhorn are proven and scalable—but can be complex to operate, tightly coupled, and opaque to extend or debug.

**AwsomeFS explores a middle-ground**:

- You already have **shared block devices** (SAN, iSCSI, NVMe-oF, loopback).
- You want **explicit coordination and consistency**—without a heavyweight replication layer.
- You need **Kubernetes-native integration**, with a pluggable, understandable stack.
- You prefer **transparency and modularity**, enabling customization and rapid iteration.
- You want **language-modern implementations** (Rust & Go), built for security and observability.

---

## 🔩 Key Features

- 🧠 **Metadata Service**: Distributed lock and coordination layer to manage volume ownership, fencing, and lifecycle.
- 💡 **fs-core (agent)**: Local daemon for volume IO coordination, device initialization, and node-level hooks.
- 📦 **CSI Plugin**: Seamless integration with Kubernetes, supporting provisioning, attachment, and lifecycle flows.
- 🧪 **Kind-based Dev Cluster**: Develop and test with raw devices simulated via loopback—ideal for rapid prototyping.
- 🔧 **Optional Kernel Module**: Enables enhanced FS primitives or custom device handling for advanced use cases.
- 🧱 **Hybrid Architecture**: Centralized metadata with decentralized IO paths = low coordination overhead.

---

## 🧰 When to Use This (vs Existing Solutions)

| Use Case | AwsomeFS | Ceph/Gluster | Local Volumes |
|----------|-------|---------------|----------------|
| Shared block device already exists | ✅ | ✅ | ❌ |
| Replication/Erasure Coding | 🚫 | ✅ | 🚫 |
| Transparent and hackable design | ✅ | ❌ | ✅ |
| Lightweight deployment | ✅ | ❌ | ✅ |
| Custom IO semantics / research | ✅ | 🚫 | 🚫 |
| Kubernetes CSI integration | ✅ | ✅ | ✅ |

---

## Comparative Overview: AwsomeFS vs Lustre vs GPFS vs Ceph

| Feature / System                        | AwsomeFS                          | Lustre                              | GPFS (Spectrum Scale)              | Ceph                                |
|----------------------------------------|-----------------------------------|-------------------------------------|-------------------------------------|-------------------------------------|
| **Shared Raw Device Support**          | ✅ Yes (via loop or SAN device)   | ✅ Yes                               | ✅ Yes                               | ❌ Not typical; object/block-based   |
| **Kubernetes CSI Integration**         | ✅ Native (built-in CSI driver)   | ❌ No official CSI                  | ⚠️ Limited / complex integration    | ✅ Mature CSI driver                 |
| **Runs in Userspace**                  | ✅ Fully                           | ❌ Kernel modules required           | ❌ Kernel modules required           | ✅ Mostly (except for kernel RBD)    |
| **POSIX-Compliant Filesystem**         | ✅ Yes                             | ✅ Yes                               | ✅ Yes                               | ⚠️ Not directly (CephFS via MDS)     |
| **Data / Metadata Separation**         | ✅ Cleanly separated               | ✅ Yes                               | ✅ Yes                               | ✅ Yes (CephFS uses MDS)             |
| **Multi-node Concurrent Access**       | ✅ Designed for it                 | ✅ Yes                               | ✅ Yes                               | ⚠️ RBD is block-only, no shared FS   |
| **Development-Friendly**              | ✅ Kind/dev-env support           | ❌ Not dev-oriented                 | ❌ Very enterprise/HPC oriented     | ✅ Yes                               |
| **Lightweight / Containerizable**      | ✅ Fully containerized            | ❌ No                                | ⚠️ Partial (heavyweight base)       | ✅ Yes                               |
| **Kernel Dependencies**                | ❌ None                            | ✅ Required                          | ✅ Required                          | ⚠️ RBD requires kernel driver        |
| **Enterprise-Readiness**               | ⚠️ In-progress (viable roadmap)   | ✅ HPC-focused                      | ✅ Enterprise-grade                 | ✅ Widely adopted                    |
| **License**                            | MIT or similar (custom)           | GPL / Open Source                   | Commercial (IBM)                    | LGPL / Open Source                  |

---

## Summary

- **AwsomeFS** gives developers and operators **Kubernetes-native shared-disk access** with modern container tooling and simplicity.
- **Lustre** and **GPFS** offer **high performance and concurrency**, but are not dev-friendly, and require **kernel-level changes** and **complex deployment**.
- **Ceph** excels in **cloud-native storage**, but it’s fundamentally object/block-based and not ideal for **raw shared disks** with filesystem-level control.

---

## Use AwsomeFS If You Want:
- To develop or run on **Kubernetes with shared block devices**
- **Dynamic PVC creation** from shared disks
- Container-based deployment
- A path toward an enterprise-grade solution with dev-first mindset



## 🏢 Enterprise-Oriented Use Cases

While experimental in nature, **AwsomeFS opens a pathway** to enterprise scenarios such as:

- **Cloud-Adjacent Deployments**: Simplify access to SAN-like devices across Kubernetes workloads with clear coordination guarantees.
- **Edge Clusters**: Where low-resource nodes share physical devices, but full-scale distributed storage is overkill.
- **Performance-Sensitive Applications**: Bypass userspace proxies and replicate logic to leverage hardware-accelerated shared storage.
- **Security & Compliance**: Use Rust and Go to build auditable, memory-safe components.
- **Custom SLA Environments**: When you control the hardware and want policy-enforced access control at the FS layer.

> 💡 You get Kubernetes-native volumes, advanced isolation, and performance—**without** depending on full-scale storage vendors or black-box systems.

---

## 🧪 Project Status

> ⚠️ **Experimental**. AwsomeFS is not production-ready yet. Ideal for prototyping, teaching, custom infra builds, and exploring FS design space.

---

## 🚀 Getting Started

```bash
make dev-env  # Sets up kind cluster with loopback-based raw devices
make build    # Builds fs-core, metadata-service, csi-driver
make deploy   # Deploys everything into your dev cluster
```

Loopback setup is handled inside `dev-env/kind-config.yaml`.

---

## 📦 Project Structure

```
/
├── cmd/                      # Entrypoints for binaries
│   ├── fs-core/              # Filesystem agent (Rust)
│   ├── metadata-service/     # Metadata coordination service (Rust)
│   └── csi-driver/           # Kubernetes CSI driver (Go)
├── internal/                 # Shared libraries and modules
├── kernel/                   # Optional kernel module (Rust + C)
├── k8s/                      # Helm charts and manifests
├── dev-env/                  # Kind config, loopback setup
├── Makefile                  # Build and development automation
```

---

## 🧠 Philosophy

AwsomeFS is not here to replace production-grade storage platforms.

Instead, it's a **transparent, modular toolkit** for developers, infra engineers, and researchers to explore how far simple coordination + shared devices can go—with a clean Kubernetes-native interface and modern dev stack.
