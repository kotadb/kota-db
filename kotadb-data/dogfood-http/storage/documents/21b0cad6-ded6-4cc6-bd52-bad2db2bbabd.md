---
tags:
- file
- kota-db
- ext_yaml
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kotadb
  labels:
    app: kotadb
    component: database
spec:
  replicas: 1
  selector:
    matchLabels:
      app: kotadb
  template:
    metadata:
      labels:
        app: kotadb
        component: database
    spec:
      containers:
      - name: kotadb
        image: ghcr.io/jayminwest/kota-db:latest
        ports:
        - containerPort: 8080
          name: mcp-server
          protocol: TCP
        - containerPort: 9090
          name: metrics
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "info"
        - name: KOTADB_DATA_DIR
          value: "/data"
        - name: KOTADB_CONFIG_FILE
          value: "/config/kotadb.toml"
        volumeMounts:
        - name: data
          mountPath: /data
        - name: config
          mountPath: /config
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 5
        securityContext:
          runAsNonRoot: true
          runAsUser: 1001
          runAsGroup: 1001
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: kotadb-data
      - name: config
        configMap:
          name: kotadb-config
      securityContext:
        fsGroup: 1001
---
apiVersion: v1
kind: Service
metadata:
  name: kotadb
  labels:
    app: kotadb
spec:
  selector:
    app: kotadb
  ports:
  - name: mcp-server
    port: 8080
    targetPort: 8080
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
  type: ClusterIP
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: kotadb-data
  labels:
    app: kotadb
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: kotadb-config
  labels:
    app: kotadb
data:
  kotadb.toml: |
    [database]
    data_directory = "/data"
    cache_size_mb = 512
    enable_wal = true
    sync_mode = "normal"
    
    [logging]
    level = "info"
    format = "json"
    log_to_file = false
    
    [performance]
    enable_metrics = true
    metrics_port = 9090
    benchmark_on_startup = false
    
    [mcp_server]
    enabled = true
    host = "0.0.0.0"
    port = 8080
    max_connections = 100
    timeout_seconds = 30
