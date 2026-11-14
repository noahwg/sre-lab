# sre-lab

A URL shortener that runs entirely on a single, cheap AWS host: web server, Postgres, and a full monitoring stack (logs, traces and metrics). Infrastructure is Terraform; configuration is Ansible over SSM, with no SSH.

## Docs

- [Getting started](docs/getting-started.md): prerequisites, `make up`, verifying, and tearing down
- [Terraform infrastructure](docs/terraform.md): modules, stacks, cost, and operational notes
- [Ansible configuration](docs/ansible.md): roles, running the playbook, and what each role does
- [Application](docs/app.md): endpoints, behavior, configuration, and local development
- [Registry and CI](docs/ci.md): ECR, GitHub OIDC, and the image build workflow

## Quick start

```sh
cp terraform/env/labs/dns/terraform.tfvars.example terraform/env/labs/dns/terraform.tfvars   # set zone_name
make up        # build the host
make destroy   # tear it down
make help      # all targets
```

Running cost is roughly $21/month while up.
