# Architecture

Visual map of the whole project. Diagrams are [Mermaid](https://mermaid.js.org/), which GitHub renders directly in Markdown. **Dashed outlines are planned, not built yet.**

## System architecture

Everything runs on one small Graviton instance. Terraform builds the AWS pieces, Ansible configures the host, and GitHub Actions publishes the app image.

```mermaid
flowchart LR
    users([Internet users])
    dev([Developer])

    subgraph aws["AWS account, us-east-1"]
        r53["Route 53<br/>A records"]
        ecr["ECR<br/>sre-lab repository"]
        s3["S3 transfer bucket<br/>Ansible file transfer"]
        ssm["SSM Session Manager"]

        subgraph vpc["VPC 10.20.0.0/16"]
            igw["Internet gateway"]
            subgraph subnet["Public subnet 10.20.1.0/24, one AZ"]
                sg{{"Security group<br/>80 and 443 in, no SSH"}}
                subgraph host["EC2 t4g.small, Elastic IP"]
                    caddy["Caddy<br/>TLS and reverse proxy"]
                    app["shortener<br/>Rust app"]
                    pg[("Postgres")]
                    vic["Victoria stack<br/>metrics, logs, traces"]
                    graf["Grafana"]
                end
                root[("Root volume<br/>gp3 20 GB")]
                tele[("Telemetry volume<br/>gp3 50 GB, separate EBS")]
            end
        end
    end

    users -->|"DNS lookup"| r53
    users -->|"HTTPS"| igw --> sg --> caddy
    caddy --> app --> pg
    app -.->|"metrics, logs, traces"| vic
    graf --> vic
    vic --- tele
    host --- root
    dev -->|"make ssm, ansible"| ssm --> host
    dev -->|"ansible files"| s3 --> host
    ecr -->|"docker pull via instance role"| host

    classDef planned stroke-dasharray: 5 5
    class caddy,pg,vic,graf planned
```

| Piece | Status | Defined in |
|---|---|---|
| VPC, subnet, security group (no SSH) | built | `terraform/modules/network` |
| Instance, Elastic IP, EBS volumes, IAM, S3 transfer bucket | built | `terraform/modules/compute` |
| Route 53 records | built | `terraform/modules/dns` |
| ECR repository, GitHub OIDC role | built | `terraform/modules/ecr`, `github_oidc` |
| Base host config (disk, swap, Docker) | built | `ansible/roles/base` |
| Shortener app (in-memory storage) | built, not deployed | `app/` |
| Caddy and TLS, Postgres, deploy role | planned | |
| Victoria stack and Grafana | planned | |

## Terraform stacks

Each stack is its own root config with local state. `make up` applies the first row in order and `make destroy` reverses it. The registry stack is separate, so tearing down the host keeps your images and the CI role.

```mermaid
flowchart LR
    subgraph host_stacks["make up and make destroy"]
        direction LR
        network --> compute --> dns --> inventory
    end
    registry["registry<br/>make registry-up"]

    network -. "VPC, subnet, SG" .-> compute
    compute -. "instance ID, Elastic IP" .-> dns
    compute -. "instance, volume, bucket" .-> inventory
    dns -. "FQDNs" .-> inventory
    inventory ==> hosts[("ansible/inventory/hosts.yml")]
    registry -. "ECR repo and CI role, no dependency on the host" .- compute
```

Dotted arrows show data passed through `terraform_remote_state`. `registry` shares nothing with the host stacks except a naming convention: the instance role may pull from the repository named `sre-lab`.

## Image build and deploy

```mermaid
sequenceDiagram
    autonumber
    actor Dev as Developer
    participant GH as GitHub Actions
    participant OIDC as GitHub OIDC
    participant STS as AWS STS
    participant ECR as ECR sre-lab
    participant Host as EC2 host

    Dev->>GH: push to main (app/ changed)
    GH->>GH: cargo test, docker build (arm64)
    GH->>OIDC: request token (sub = repo + branch)
    OIDC-->>GH: signed JWT
    GH->>STS: AssumeRoleWithWebIdentity (JWT + role ARN)
    STS-->>GH: temporary credentials, if trust policy matches
    GH->>ECR: push shortener-commit-sha
    Note over Dev,Host: Planned: Ansible logs in to ECR and runs the chosen tag
    Dev->>Host: make configure (app_image_tag)
    Host->>ECR: docker pull via instance role
```

Pull requests run the tests and build the image but skip the AWS steps, so they never push.

## How the host is managed

There is no SSH and no inbound management port. Both the shell and Ansible go out through SSM.

```mermaid
flowchart LR
    laptop(["Laptop<br/>aws cli, session-manager-plugin, Ansible"])
    ssm["SSM Session Manager"]
    s3["S3 transfer bucket<br/>objects expire after 1 day"]
    agent["SSM agent on the host<br/>outbound 443 only"]

    laptop -->|"make ssm: shell"| ssm
    laptop -->|"make configure: tasks"| ssm
    laptop -->|"module files"| s3
    ssm <--> agent
    agent -->|"fetch files"| s3
```

## Observability data flow (planned)

Metrics are pulled and traces and logs are pushed. The app is the only service that needs instrumenting.

```mermaid
flowchart LR
    app["shortener"]
    pgx["postgres_exporter"]
    nodex["node_exporter"]
    agent["vmagent"]
    vm[("VictoriaMetrics")]
    vl[("VictoriaLogs")]
    vt[("VictoriaTraces")]
    alert["vmalert"]
    graf["Grafana"]

    app -->|"/metrics"| agent
    pgx --> agent
    nodex --> agent
    agent -->|"scrape and store"| vm
    app -->|"OTLP traces"| vt
    app -->|"stdout logs"| vl
    vm --> alert
    vm --> graf
    vl --> graf
    vt --> graf

    classDef planned stroke-dasharray: 5 5
    class pgx,nodex,agent,vm,vl,vt,alert,graf planned
```

All of it is stored on the separate telemetry volume so it can be resized or snapshotted without touching the OS disk.
