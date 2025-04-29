FROM golang:1.23 as builder

WORKDIR /app
COPY . .
RUN CGO_ENABLED=0 go build -o csi-awsomefs main.go

FROM gcr.io/distroless/static
COPY --from=builder /app/csi-awsomefs /
CMD ["/csi-awsomefs"]
