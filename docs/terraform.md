# Terraform infrastructure

One cheap host runs the whole stack (web server, Postgres, logs/traces/metrics). Terraform builds the host and everything around it; Ansible configures it.

## Layout

```
terraform/
  modules/              reusable building blocks
    network/            VPC, subnet, routing, security group
    compute/            instance, Elastic IP, IAM, telemetry volume, transfer bucket
    dns/                Route 53 A records
    ecr/                container image repository
    github_oidc/        IAM role GitHub Actions assumes to push images
  env/labs/             one root config ("stack") per directory, each with its own state
    network/  compute/  dns/  inventory/   applied by make up / make destroy
    registry/                              ECR + CI role, applied separately (make registry-up)
```

Modules hold the resources. Each stack under `env/labs/` is a thin wrapper that calls one module (the `inventory` stack calls none). Stacks apply in this order and share data through `terraform_remote_state` (local state files):

```
network ──► compute ──► dns ──► inventory
```

Destroy runs in reverse. `make up` / `make destroy` handle the ordering.

## Architecture

```
Internet
   │  80/443 only
[Internet gateway] ── [Public subnet 10.20.1.0/24, 1 AZ]
                              │
                      [Security group: no port 22]
                              │
                  [t4g.small, Elastic IP]──── Route 53 A records
                     │              │
              root gp3 20 GB   telemetry gp3 50 GB (separate EBS)

Management: SSM Session Manager (outbound 443 only) + S3 bucket for Ansible file transfer
```

## Modules

Each module lives in `terraform/modules/<name>/` with `main.tf`, `variables.tf` and `outputs.tf`. A stack under `env/labs/` supplies the inputs and re-exports the outputs it needs.

### network

Builds the minimum networking a single public host needs. Nothing here costs money.

| Resource | Details |
|---|---|
| `aws_vpc` | `10.20.0.0/16`, DNS support and hostnames enabled |
| `aws_internet_gateway` | The VPC's route to the internet |
| `aws_subnet` (public) | `10.20.1.0/24` in a single AZ; does **not** auto-assign public IPs (the Elastic IP provides one) |
| `aws_route_table` + association | Sends `0.0.0.0/0` to the internet gateway, which is what makes the subnet public |
| `aws_security_group` | Name-prefixed with `create_before_destroy`, so it can be replaced without a naming collision |
| `aws_vpc_security_group_ingress_rule` (x2 per CIDR) | TCP 80 and 443 from each CIDR in `web_ingress_cidrs` |
| `aws_vpc_security_group_egress_rule` | All outbound (SSM, package repos, ACME, image pulls) |

Design notes:

- **No port 22.** There is deliberately no SSH rule. SSM Session Manager works over an outbound HTTPS connection from the instance's agent, so nothing has to reach in.
- **No NAT gateway, private subnets or VPC endpoints.** A public subnet plus an Elastic IP is enough for SSM and outbound traffic. A NAT gateway alone would cost about $32/mo.
- **One AZ.** The telemetry EBS volume must live in the same AZ as the instance, so the subnet's AZ is passed through to `compute`.

| Inputs | Default |
|---|---|
| `name` | required |
| `availability_zone` | required |
| `vpc_cidr` | `10.20.0.0/16` |
| `public_subnet_cidr` | `10.20.1.0/24` |
| `web_ingress_cidrs` | `["0.0.0.0/0"]` |

Outputs: `vpc_id`, `subnet_id`, `availability_zone`, `security_group_id`.

### compute

Builds the host and everything attached to it. This is where nearly all the cost is.

| Resource | Details |
|---|---|
| `aws_ssm_parameter` (data) | Looks up the latest Amazon Linux 2023 **arm64** AMI, so the instance always starts from a current image |
| `aws_instance` | `t4g.small` (Graviton), no key pair, no auto-assigned public IP, **IMDSv2 required** (hop limit 2 so containers can reach metadata), encrypted 20 GB gp3 root volume. `ignore_changes = [ami]` stops a newer AMI from replacing the host |
| `aws_eip` | Stable public address for DNS |
| `aws_ebs_volume` (telemetry) | Separate encrypted 50 GB gp3 volume in the instance's AZ |
| `aws_volume_attachment` | Attaches it at `/dev/sdf` with `stop_instance_before_detaching` |
| `aws_iam_role` + `aws_iam_instance_profile` | EC2 assume-role; has the `AmazonSSMManagedInstanceCore` managed policy attached |
| `aws_iam_role_policy` | Read/write/delete on the transfer bucket and list on the bucket itself |
| `aws_s3_bucket` (+ public access block, SSE, lifecycle) | Transfer bucket for Ansible's `community.aws.aws_ssm` connection, which moves files through S3. Private, AES256-encrypted, objects expire after 1 day, `force_destroy` so `make destroy` doesn't fail on leftovers. Name includes the account ID for global uniqueness |

Design notes:

- **Separate telemetry volume.** Logs, metrics and traces can be sized, snapshotted and resized independently of the OS disk, and the data survives replacing the instance. It is deleted by `make destroy`.
- **Device naming.** On Nitro instances `/dev/sdf` shows up as `/dev/nvmeXn1`, so Ansible resolves the disk by volume ID (`/dev/disk/by-id/nvme-Amazon_Elastic_Block_Store_<id>`).
- **SSM only.** No key pair exists, so the only way in is SSM (or Ansible over SSM).
- The instance type must be arm64 because the AMI is arm64.

| Inputs | Default |
|---|---|
| `name`, `subnet_id`, `availability_zone`, `security_group_ids` | required |
| `instance_type` | `t4g.small` |
| `root_volume_size_gb` | `20` |
| `telemetry_volume_size_gb` | `50` |
| `telemetry_volume_type` | `gp3` |
| `telemetry_device_name` | `/dev/sdf` |

Outputs: `instance_id`, `public_ip`, `private_ip`, `telemetry_volume_id`, `telemetry_device_name`, `ssm_transfer_bucket`.

### dns

Points your domain names at the host. Small, and costs nothing beyond the hosted zone you already have.

| Resource | Details |
|---|---|
| `aws_route53_zone` (data) | Looks up an **existing** public hosted zone by `zone_name`. It doesn't create the zone, since registering a domain and delegating nameservers is a one-time manual step |
| `aws_route53_record` (one per `record_names` entry) | A record, TTL 300 by default, pointing at the Elastic IP |

How names are built: `"@"` is the zone apex; anything else becomes `<name>.<zone_name>`. So with `zone_name = "example.com"`, `"grafana"` becomes `grafana.example.com`, and `"grafana.lab"` becomes `grafana.lab.example.com`. This also covers a subdomain inside a parent zone: use `["lab"]` rather than `"@"`.

| Inputs | Default |
|---|---|
| `zone_name` | required |
| `ip_address` | required |
| `record_names` | `["@"]` |
| `ttl` | `300` |

Outputs: `fqdns` (map of record name to FQDN) and `zone_id`. The `dns` stack adds `primary_fqdn`, the FQDN of the first `record_names` entry.
### ecr

One shared private repository (named `name`, default `sre-lab`) for all of the project's images. Tags are **immutable**, images are scanned on push and encrypted (AES256), and a lifecycle policy expires everything beyond the newest `keep_image_count` (default 30). `force_delete` lets `destroy` remove a repository that still holds images.

Outputs: `repository_url`, `repository_arn`, `repository_name`.

### github_oidc

Lets GitHub Actions push images without stored AWS keys. It creates the GitHub OIDC identity provider (or reuses one via `existing_oidc_provider_arn`, since an account can have only one) and the role `<name>-github-ci`. The role's trust policy only accepts `repo:<github_repository>:ref:<ref>` for the refs in `github_refs` (default `refs/heads/main`), so pull requests can't assume it. Its permissions are limited to pushing to the one ECR repository.

Output: `role_arn`.

The `compute` module also grants the instance role **pull-only** access to the `<name>` ECR repository, so the host can `docker pull` with no credentials.

## The registry stack

`env/labs/registry/` wires `ecr` and `github_oidc` together. It is **not** part of `make up` / `make destroy`, so tearing down the host doesn't delete your images or the CI role. Run it with `make registry-up` and remove it with `make registry-destroy`. It prints a `github_variables` output with the `gh variable set` commands for the repository variables a workflow needs.

If `make registry-up` fails with `EntityAlreadyExists` for the OIDC provider, the account already has one: set `existing_oidc_provider_arn` in `registry/terraform.tfvars` (`arn:aws:iam::<account>:oidc-provider/token.actions.githubusercontent.com`) and apply again.

## The inventory stack

Reads the `compute` and `dns` outputs, renders `inventory/templates/inventory.yml.tftpl`, and writes `ansible/inventory/hosts.yml` (git-ignored, regenerated on every apply). It's also available via `make inventory`. Per host it provides:

| Variable | Purpose |
|---|---|
| `ansible_host` | Instance ID (connection is `community.aws.aws_ssm`, not SSH) |
| `ansible_aws_ssm_region`, `ansible_aws_ssm_bucket_name` | SSM connection settings |
| `public_ip`, `primary_fqdn`, `fqdns` | Addressing; `primary_fqdn` is the first `record_names` entry |
| `telemetry_volume_id`, `telemetry_mount_point` | Where Ansible finds and mounts the telemetry disk (default `/var/lib/telemetry`) |

## Configuration

| Variable | Stack | Default |
|---|---|---|
| `region`, `name` | all | `us-east-1`, `sre-lab` |
| `zone_name`, `record_names` | dns | required, `["@"]` |
| `instance_type` | compute | `t4g.small` |
| `root_volume_size_gb`, `telemetry_volume_size_gb` | compute | `20`, `50` |
| `web_ingress_cidrs` | network | `["0.0.0.0/0"]` |
| `telemetry_mount_point`, `write_inventory_file` | inventory | `/var/lib/telemetry`, `true` |

`region` and `name` are declared in every stack and can be shared via `terraform/env/labs/common.tfvars`.

## Cost

Estimates for us-east-1 at on-demand list prices; confirm with the AWS Pricing Calculator.

| Item | ~Monthly |
|---|---|
| t4g.small ($0.0168/hr) | $12.26 |
| Root gp3, 20 GB | $1.60 |
| Telemetry gp3, 50 GB | $4.00 |
| Public IPv4 / Elastic IP ($0.005/hr) | $3.65 |
| VPC, IGW, SG, IAM, SSM, S3 bucket, Route 53 records | ~$0 |
| **Total** | **~$21.5** |

Data transfer out is free for the first 100 GB/mo, then ~$0.09/GB. A Route 53 hosted zone is $0.50/mo, but this repo only uses an existing one. A stopped instance still pays for EBS and the IP (~$9/mo).

## Operational notes

- **State is local.** Each stack keeps `terraform.tfstate` in its own directory (git-ignored). Losing it orphans the resources, so back it up or move to an S3 backend later (`versions.tf` has no backend block yet, and the three `terraform_remote_state` blocks would need updating).
- **`make destroy` deletes the telemetry volume.** That's intentional for a throwaway lab.
- **No TLS yet.** Ports 80/443 are open, but certificates are issued on the host in the Ansible phase (e.g. Let's Encrypt via a reverse proxy). ACM can't attach to a bare EC2 instance.
- **Ports 80/443 are open to the world** by default because this is a public service. Narrow them with `web_ingress_cidrs`.
- Commit each stack's `.terraform.lock.hcl`; never commit `*.tfstate`, `terraform.tfvars` or `hosts.yml` (all git-ignored).
