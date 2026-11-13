# Getting started

How to stand up the lab host from scratch, verify it, and tear it down.

## Prerequisites

| Tool | Why |
|---|---|
| Terraform >= 1.5 | Builds the infrastructure |
| AWS CLI v2, configured with credentials | Terraform and SSM use it |
| [`session-manager-plugin`](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html) | `make ssm` and Ansible connect through it |
| Python 3 | `make deps` builds a `.venv` with Ansible and boto3 from `ansible/requirements.txt` |
| `make` | Drives the stacks in order |
| A Route 53 **public hosted zone** | The `dns` stack looks it up; it doesn't create it |

Your AWS identity needs permission to manage EC2, VPC, IAM roles/profiles, S3, Route 53 records and SSM.

```sh
make deps    # .venv + Ansible collections (needed before make ping / configure)
```

## 1. Configure

Only the `dns` stack needs input:

```sh
cp terraform/env/labs/dns/terraform.tfvars.example terraform/env/labs/dns/terraform.tfvars
```

Edit `zone_name` and `record_names`. Two common setups:

```hcl
# Apex of a zone you own
zone_name    = "example.com"
record_names = ["@", "grafana"]          # example.com, grafana.example.com

# A subdomain inside the parent zone (don't use "@" here)
zone_name    = "example.com"
record_names = ["lab", "grafana.lab"]    # lab.example.com, grafana.lab.example.com
```

The first entry in `record_names` becomes the host's `primary_fqdn`.

Optional: copy `terraform/env/labs/common.tfvars.example` to `common.tfvars` to override the shared `region` / `name` (defaults: `us-east-1`, `sre-lab`). Other knobs (instance type, volume sizes, ingress CIDRs) live in each stack's `variables.tf`.

## 2. Build

```sh
make up
```

This applies `network → compute → dns → inventory`, prompting for approval at each stack (`AUTO_APPROVE=1` skips the prompts). When it finishes it prints an SSM session command, and the Ansible inventory is written to `ansible/inventory/hosts.yml` (git-ignored).

Run a single stack with `make up STACK=compute`. Run `make help` for all targets.

## 3. Verify

```sh
# SSM sees the instance (can take a minute or two after boot)
aws ssm describe-instance-information \
  --query 'InstanceInformationList[].[InstanceId,PingStatus]' --output table

make ssm                                   # shell on the host, no SSH
lsblk                                      # root disk + an unmounted ~50G telemetry disk

nc -vz -w 3 <public_ip> 22                 # times out: no SSH rule
nc -vz -w 3 <public_ip> 80                 # refused: allowed, but nothing listening yet
dig +short <primary_fqdn>                  # returns the Elastic IP

make ping                                  # pong = full chain works
```

The telemetry disk is attached but unformatted. Mounting it is the Ansible phase's job.

## 4. Tear down

```sh
make destroy
```

Destroys in reverse order (`inventory → dns → compute → network`). **This deletes the telemetry volume and everything on it.**

## Cost

About **$21-22/month** while running (t4g.small ~$12, EBS ~$6, public IPv4 ~$3.65). A stopped instance still pays for EBS and the IP (~$9/mo). `make destroy` stops all of it. Details in [terraform.md](terraform.md#cost).

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `Missing .../dns/terraform.tfvars` | Step 1 not done |
| `dns` fails: no matching Route 53 zone | `zone_name` isn't an existing **public** hosted zone in this account |
| `compute` can't plan on a fresh account | Later stacks read earlier stacks' state; apply `network` first (`make up` does this) |
| Instance never appears in SSM | Wait a couple of minutes; check outbound 443 and the instance's IAM profile |
| `ansible ping` fails | Missing `session-manager-plugin` or collections, or your credentials can't read/write the transfer bucket |
