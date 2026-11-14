# Container registry and CI

The app image is built and pushed to a private ECR repository by GitHub Actions. The host pulls it from there with its instance role.

```
git push to main ──► GitHub Actions (arm64 runner)
                       ├─ cargo test
                       └─ docker build ──► ECR  sre-lab:shortener-<full commit SHA>
                                                      │
              EC2 host (instance role, pull-only) ◄───┘   (Ansible deploys a chosen SHA)
```

## Terraform: the `registry` stack

`terraform/env/labs/registry/` calls two modules:

| Module | Resources |
|---|---|
| `ecr` | One shared repository, `sre-lab`, with **immutable tags** (a tag can never be repointed), scan on push, AES256 encryption, and a lifecycle policy that keeps the last 30 images across all images |
| `github_oidc` | GitHub OIDC identity provider (optional, one per account) and an IAM role `sre-lab-github-ci` that can push to that one repository |

**No access keys are stored in GitHub.** The workflow trades its short-lived OIDC token for the role. The role's trust policy only accepts the `noahwg/sre-lab` repository on `refs/heads/main`, so pull requests can't assume it.

The `compute` module also gets a pull-only ECR policy on the instance role, scoped to the `sre-lab` repository.

The registry stack is **separate from `make up` / `make destroy`**, so tearing down the host doesn't delete your images or the CI role.

```sh
make registry-up        # one-time
make registry-destroy   # deletes the repo and its images
```

## One-time setup

1. `make registry-up`. If your account already has a GitHub OIDC provider, apply fails with "EntityAlreadyExists". Set `existing_oidc_provider_arn` in `terraform/env/labs/registry/terraform.tfvars` and run it again.
2. Run the commands from the `github_variables` output so the workflow knows the role and repo:
   ```sh
   terraform -chdir=terraform/env/labs/registry output -raw github_variables | sh
   ```
3. Run `make up STACK=compute` to add the pull policy to the instance role.

## The workflow (`.github/workflows/app.yml`)

| Event | What runs |
|---|---|
| Pull request touching `app/` | `cargo test`, then a `docker build` that is **not pushed** |
| Push to `main` touching `app/` | tests, then build and **push** `<registry>/sre-lab:shortener-<github.sha>` |

Both jobs run on GitHub's `ubuntu-24.04-arm` runners, so the build is native arm64 with no QEMU emulation. Those runners are free for public repositories. If the repo is private, switch to `ubuntu-latest` with QEMU (slower).

One repository holds every image, told apart by the tag prefix: `shortener-<full commit SHA>`. A future image would use its own prefix (for example `worker-<sha>`). The prefix matters because tags are immutable, so two images built from the same commit can't share a bare SHA tag.

There is no `latest`: every deploy names an exact build, and a rollback is just a different SHA. The lifecycle policy counts all images together, so raise `keep_image_count` if you add many.
