# Project variables
REPO_NAME := awsomefs
REGISTRY ?= awsomefs
VERSION  ?= latest

# Paths
FS_CORE_DIR := cmd/fs-core
METADATA_SERVICE_DIR := cmd/metadata-service
CSI_DRIVER_DIR := cmd/csi-awsomefs
KERNEL_MODULE_DIR := kernel/fs_driver

# Docker image names
FS_CORE_IMAGE := $(REGISTRY)/fs-core:$(VERSION)
METADATA_IMAGE := $(REGISTRY)/metadata-service:$(VERSION)
CSI_DRIVER_IMAGE := $(REGISTRY)/csi-awsomefs:$(VERSION)

# Kernel module paths
KERNEL_MODULE := $(KERNEL_MODULE_DIR)/fs_module.ko
KERNEL_MODULE_BUILD := $(KERNEL_MODULE_DIR)/build

##@ General

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

.PHONY: build
build:
	cargo build --manifest-path $(FS_CORE_DIR)/Cargo.toml --release
	cargo build --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml --release
	cd $(CSI_DRIVER_DIR) && go build -o target/csi-awsomefs main.go

.PHONY: test
test: ## test all components
	cargo test --manifest-path $(FS_CORE_DIR)/Cargo.toml
	cargo test --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml
	cd $(CSI_DRIVER_DIR) && go test ./...

.PHONY: build
docker: docker-fs-core docker-metadata-service docker-csi-driver ## build docker image of all components

.PHONY: docker-fs-core
docker-fs-core: ## build docker image of fs-core
	docker build -t $(FS_CORE_IMAGE) -f $(FS_CORE_DIR)/Dockerfile .

.PHONY: docker-metadata-service
docker-metadata-service: ## build docker image of metadata-service
	docker build -t $(METADATA_IMAGE) -f $(METADATA_SERVICE_DIR)/Dockerfile .

.PHONY: docker-csi-driver
docker-csi-driver: ## build docker image of csi-driver
	docker build -t $(CSI_DRIVER_IMAGE) $(CSI_DRIVER_DIR)

##@ Kernel Module

.PHONY: kernel-module
kernel-module: ## Build the kernel module
	make -C $(KERNEL_MODULE_DIR) all

.PHONY: install-kernel-module
install-kernel-module: ## Install the kernel module to system (requires root privileges)
	sudo insmod $(KERNEL_MODULE)

.PHONY: uninstall-kernel-module
uninstall-kernel-module: ## Unload the kernel module from system (requires root privileges)
	sudo rmmod $(basename $(notdir $(KERNEL_MODULE)))

.PHONY: cleanup-kernel-module
cleanup-kernel-module: ## Cleanup the generated files from the kernel module
	make -C $(KERNEL_MODULE_DIR) clean


##@ Dev environment

.PHONY: dev-env
dev-env: docker loopback-device kind-cluster kind-load deploy ## Set up all the things (create kind cluster build and deploy to it)
	kubectl apply -f examples/

.PHONY: loopback-device
loopback-device: ## Create fake shared device for kind cluster
	dd if=/dev/zero of=/tmp/loopback-device bs=1M count=1024

.PHONY: kind-cluster
kind-cluster: ## Create kind-cluster
	kind create cluster --name $(REPO_NAME) --config dev-env/kind-config.yaml || true

.PHONY: kind-load
kind-load: ## Load images to kind-cluster
	kind load docker-image $(FS_CORE_IMAGE) --name $(REPO_NAME)
	kind load docker-image $(METADATA_IMAGE) --name $(REPO_NAME)
	kind load docker-image $(CSI_DRIVER_IMAGE) --name $(REPO_NAME)

.PHONY: deploy
deploy: ## Deploy all components to the kind cluster
	kubectl apply -f k8s/metadata-service/
	kubectl apply -f k8s/fs-core/
	kubectl apply -f k8s/csi-driver/

.PHONY: undeploy
undeploy: ## Remove all deployed components
	kubectl delete -f k8s/csi-driver/ || true
	kubectl delete -f k8s/fs-core/ || true
	kubectl delete -f k8s/metadata-service/ || true

.PHONY: clean
clean: ## Cleanup everything
	cargo clean --manifest-path $(FS_CORE_DIR)/Cargo.toml
	cargo clean --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml
	cd $(CSI_DRIVER_DIR) && go clean
	kind delete cluster --name $(REPO_NAME)
	rm /tmp/loopback-device
