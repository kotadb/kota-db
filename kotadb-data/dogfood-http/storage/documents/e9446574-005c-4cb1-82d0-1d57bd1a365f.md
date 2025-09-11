---
tags:
- file
- kota-db
- ext_yml
---
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "kotadb-rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'kotadb'
    static_configs:
      - targets: ['kotadb:9090']
    scrape_interval: 10s
    metrics_path: /metrics
    
  - job_name: 'kotadb-mcp'
    static_configs:
      - targets: ['kotadb:8080']
    scrape_interval: 15s
    metrics_path: /mcp/metrics
