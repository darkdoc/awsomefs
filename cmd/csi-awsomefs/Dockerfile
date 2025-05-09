FROM golang:1.23 as builder

WORKDIR /app
COPY . .
RUN CGO_ENABLED=0 go build -o csi-awsomefs main.go

FROM registry.redhat.io/ubi9/ubi-minimal
# install mountpoint binary
RUN microdnf install -y util-linux
COPY --from=builder /app/csi-awsomefs /
ENTRYPOINT ["/csi-awsomefs"]
