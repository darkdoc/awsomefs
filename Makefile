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

.PHONY: all build test docker kind-cluster clean kernel-module install-kernel-module

all: build

##@ General

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

build: 
	cargo build --manifest-path $(FS_CORE_DIR)/Cargo.toml --release
	cargo build --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml --release
	cd $(CSI_DRIVER_DIR) && go build -o csi-awsomefs main.go
	make -C $(KERNEL_MODULE_DIR) 


test: ## test all components
	cargo test --manifest-path $(FS_CORE_DIR)/Cargo.toml
	cargo test --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml
	cd $(CSI_DRIVER_DIR) && go test ./...


docker: docker-fs-core docker-metadata-service docker-csi-driver ## build docker image of all components

docker-fs-core: ## build docker image of fs-core
	docker build -t $(FS_CORE_IMAGE) $(FS_CORE_DIR)

docker-metadata-service: ## build docker image of metadata-service
	docker build -t $(METADATA_IMAGE) $(METADATA_SERVICE_DIR)

docker-csi-driver: ## build docker image of csi-driver
	docker build -t $(CSI_DRIVER_IMAGE) $(CSI_DRIVER_DIR)

##@ Kernel Module

kernel-module: ## Build the kernel module
	make -C $(KERNEL_MODULE_DIR) all

install-kernel-module: ## Install the kernel module to system (requires root privileges)
	sudo insmod $(KERNEL_MODULE)

uninstall-kernel-module: ## Unload the kernel module from system (requires root privileges)
	sudo rmmod $(basename $(notdir $(KERNEL_MODULE)))

##@ Dev environment
loopback-device: ## Create fake shared device for kind cluster
	dd if=/dev/zero of=/tmp/loopback-device bs=1M count=1024

kind-cluster: loopback-device ## Create kind-cluster
	kind create cluster --name $(REPO_NAME) --config dev-env/kind-config.yaml || true

kind-load: ## Load images to kind-cluster
	kind load docker-image $(FS_CORE_IMAGE) --name $(REPO_NAME)
	kind load docker-image $(METADATA_IMAGE) --name $(REPO_NAME)
	kind load docker-image $(CSI_DRIVER_IMAGE) --name $(REPO_NAME)

# kind-unload:
# 	cluster=$(REPO_NAME)
# 	containers="$(REPO_NAME)-control-plane $(REPO_NAME)-worker $(REPO_NAME)-worker2 $(REPO_NAME)-worker3"
# 	for container in $(REPO_NAME)-control-plane $(REPO_NAME)-worker $(REPO_NAME)-worker2 $(REPO_NAME)-worker3; do \
#   		echo "Cleaning import images in container: $$container"; \
# 		docker exec -it $$container bash -c 'ctr -n=k8s.io images ls | grep $(FS_CORE_IMAGE)'; \
# docker exec -it "$(container)" bash -c 'ctr -n=k8s.io images ls | grep $(FS_CORE_IMAGE) | cut -f1 -d" " | xargs ctr -n=k8s.io images rm'
# 	done


deploy: kind-load ## Deploy all components to the kind cluster
	kubectl apply -f k8s/metadata-service/
	kubectl apply -f k8s/fs-core/
	kubectl apply -f k8s/csi-driver/



undeploy: ## Remove all deployed components
	kubectl delete -f k8s/csi-driver/ || true
	kubectl delete -f k8s/fs-core/ || true
	kubectl delete -f k8s/metadata-service/ || true

clean: ## Cleanup
	cargo clean --manifest-path $(FS_CORE_DIR)/Cargo.toml
	cargo clean --manifest-path $(METADATA_SERVICE_DIR)/Cargo.toml
	cd $(CSI_DRIVER_DIR) && go clean
	make -C $(KERNEL_MODULE_DIR) clean
	kind delete cluster --name $(REPO_NAME)
	rm /tmp/loopback-device

