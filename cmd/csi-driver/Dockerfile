FROM golang:1.23 as builder

WORKDIR /app
COPY . .
RUN go build -o csi-driver main.go

FROM gcr.io/distroless/static
COPY --from=builder /app/csi-driver /csi-driver
ENTRYPOINT ["/csi-driver"]
