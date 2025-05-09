package driver

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/container-storage-interface/spec/lib/go/csi"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (d *Driver) CreateVolume(ctx context.Context, req *csi.CreateVolumeRequest) (*csi.CreateVolumeResponse, error) {
	// --- Check if basePath is a mountpoint ---
	if err := checkMountpoint(baseMountPath); err != nil {
		return nil, fmt.Errorf("base path %s is not mounted: %v", baseMountPath, err)
	}

	volID := req.GetName()
	if volID == "" {
		return nil, status.Error(codes.InvalidArgument, "volume name is required")
	}

	capacity := req.GetCapacityRange().GetRequiredBytes()
	if capacity == 0 {
		capacity = int64(1 << 30) // Default to 1 GiB
	}
	volumePath := filepath.Join(baseMountPath, "volumes", volID)

	log.Printf("CreateVolume called for: %s (capacity: %d bytes)", volID, capacity)

	// Ensure volume directory exists
	if _, err := os.Stat(volumePath); err == nil {
		log.Printf("Volume %s already exists at %s", volID, volumePath)
	} else if os.IsNotExist(err) {
		if err := os.MkdirAll(volumePath, 0755); err != nil {
			return nil, status.Errorf(codes.Internal, "failed to create volume directory: %v", err)
		}
		log.Printf("Created volume directory: %s", volumePath)
	} else {
		return nil, status.Errorf(codes.Internal, "failed to stat volume path: %v", err)
	}

	// TODO: Actually provision the volume somewhere (e.g., mkdir, thin file, etc.)
	volume := &csi.Volume{
		VolumeId:      volID,
		CapacityBytes: capacity, // stubbed
		VolumeContext: map[string]string{
			"mountPath": volumePath,
		},
	}

	return &csi.CreateVolumeResponse{Volume: volume}, nil
}

func (d *Driver) DeleteVolume(ctx context.Context, req *csi.DeleteVolumeRequest) (*csi.DeleteVolumeResponse, error) {

	// --- Sanity check: is baseMountPath mounted? ---
	if err := checkMountpoint(baseMountPath); err != nil {
		return nil, fmt.Errorf("base mount path %s is not mounted: %v", baseMountPath, err)
	}

	volID := req.GetVolumeId()
	if volID == "" {
		return nil, fmt.Errorf("volume ID is required")
	}

	volumePath := filepath.Join(baseMountPath, "volumes", volID)

	if _, err := os.Stat(volumePath); os.IsNotExist(err) {
		log.Printf("DeleteVolume: volume path %s does not exist, treating as deleted", volumePath)
		// Volume already deleted, return success
		return &csi.DeleteVolumeResponse{}, nil
	}

	if err := os.RemoveAll(volumePath); err != nil {
		return nil, fmt.Errorf("failed to delete volume directory %s: %v", volumePath, err)
	}

	log.Printf("DeleteVolume: successfully deleted %s", volumePath)
	return &csi.DeleteVolumeResponse{}, nil
}

func (d *Driver) ControllerGetCapabilities(ctx context.Context, req *csi.ControllerGetCapabilitiesRequest) (*csi.ControllerGetCapabilitiesResponse, error) {
	return &csi.ControllerGetCapabilitiesResponse{
		Capabilities: []*csi.ControllerServiceCapability{
			{
				Type: &csi.ControllerServiceCapability_Rpc{
					Rpc: &csi.ControllerServiceCapability_RPC{
						Type: csi.ControllerServiceCapability_RPC_CREATE_DELETE_VOLUME,
					},
				},
			},
		},
	}, nil
}

func (d *Driver) ControllerPublishVolume(
	ctx context.Context,
	req *csi.ControllerPublishVolumeRequest,
) (*csi.ControllerPublishVolumeResponse, error) {
	return &csi.ControllerPublishVolumeResponse{}, nil
}

func (d *Driver) ControllerUnpublishVolume(
	ctx context.Context,
	req *csi.ControllerUnpublishVolumeRequest,
) (*csi.ControllerUnpublishVolumeResponse, error) {
	return &csi.ControllerUnpublishVolumeResponse{}, nil
}

func checkMountpoint(path string) error {
	cmd := exec.Command("mountpoint", "-q", path)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("not a mountpoint or inaccessible: %v", err)
	}
	return nil
}
