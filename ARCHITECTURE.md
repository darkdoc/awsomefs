
# Architecture Overview

This document outlines the architecture of the hybrid distributed filesystem designed for Kubernetes environments with shared block devices across nodes.

## Components

### 1. CSI Driver (`csi-driver/` - Go)
Implements the Container Storage Interface (CSI) to enable Kubernetes to provision and manage volumes on the distributed filesystem.

- Language: Go
- Interface between Kubernetes and the custom storage system.
- Coordinates volume lifecycle: create, mount, unmount, delete.

### 2. Metadata Service (`metadata-service/` - Rust)
Central coordination service for metadata operations.

- Language: Rust
- Responsible for tracking volume ownership, locations, consistency, and access control.
- Provides APIs for `fs-core` to interact with metadata.

### 3. Filesystem Core (`fs-core/` - Rust)
Runs on every node and handles the data path of filesystem operations.

- Language: Rust
- Communicates with the metadata service to get metadata for volumes.
- Handles mounting, reading/writing blocks, caching, and fencing.
- Mounts raw devices (e.g., `/dev/sdX`) and exposes them to containers.

### 4. Kernel Module (`kernel-module/` - C)
Optional module for performance-critical paths.

- Language: C
- Provides enhanced access to block devices and caching.
- Communicates with `fs-core` or exposes a VFS interface for faster I/O.

### 5. Dev Environment (`dev-env/`)
Development and test environment using `kind` (Kubernetes in Docker).

- Local testbed using loopback devices as shared disks.
- Includes Kind config with custom raw block devices mapped to each node.
- Dockerfiles for each component to run inside Kubernetes.

### 6. CI/CD & Build (`Makefile`, `Dockerfiles`, etc.)
- Manages builds, tests, kernel compilation, Docker images, and deployment.
- Parameterized and modular for easy customization.

## Communication Flow

1. **Kubernetes** calls CSI driver when a pod requests storage.
2. **CSI driver** calls metadata service to create/attach volume.
3. **Metadata service** records volume info and assigns to a node.
4. **CSI driver** on the node launches or instructs `fs-core` to mount the device.
5. **fs-core** mounts the raw device, consults metadata service, and makes volume available to the pod.
6. **Kernel module** may assist `fs-core` with performance optimizations if available.

## Design Goals

- **Hybrid approach**: User space logic with optional kernel acceleration.
- **Data integrity**: Coordinated access via metadata service to prevent corruption.
- **Flexibility**: Suitable for on-prem, cloud, or edge deployments.
- **Compatibility**: Full Kubernetes integration via CSI.

## Advantages

- Fine-grained control over storage logic.
- Tailored performance and coordination mechanisms.
- Enterprise features like fencing, data locality, and encryption.
- Easier to debug and evolve than kernel-only solutions.

