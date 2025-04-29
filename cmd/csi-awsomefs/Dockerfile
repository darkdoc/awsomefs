FROM golang:1.23 as builder

WORKDIR /app
COPY . .
RUN go build -o csi-awsomefs main.go

FROM gcr.io/distroless/static
COPY --from=builder /app/csi-awsomefs /csi-awsomefs
ENTRYPOINT ["/csi-awsomefs"]
