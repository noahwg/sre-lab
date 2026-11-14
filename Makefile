ENV          := terraform/env/labs
ORDER        := network compute dns inventory
# Registry (ECR + GitHub OIDC) and secrets (SSM parameters) have their own lifecycle:
# `make up` / `make destroy` leave them alone.
ALL_STACKS   := registry secrets $(ORDER)
REVERSE      := inventory dns compute network
STACK        ?=
AUTO_APPROVE ?=
APPROVE_FLAG := $(if $(AUTO_APPROVE),-auto-approve,)
# Optional shared vars (region/name) passed to every stack if the file exists.
COMMON_VARS  := $(if $(wildcard $(ENV)/common.tfvars),-var-file=../common.tfvars,)

UP_STACKS    := $(or $(STACK),$(ORDER))
DOWN_STACKS  := $(or $(STACK),$(REVERSE))

# $(call tf,<stack>,<args>)
tf = terraform -chdir=$(ENV)/$(1) $(2)

.DEFAULT_GOAL := help
ANSIBLE_DIR  := ansible
VENV         := .venv
# Ansible targets use the repo-local virtualenv.
export PATH  := $(abspath $(VENV))/bin:$(PATH)
# macOS: Ansible forks workers, and the ObjC runtime aborts forked children by default.
export OBJC_DISABLE_INITIALIZE_FORK_SAFETY := YES
ANSIBLE_ARGS ?=

.PHONY: deploy secrets-up secrets-destroy registry-up registry-destroy app-test app-image help up destroy plan init fmt validate inventory ssm check-vars venv deps configure ping check

help: ## Show targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'
	@echo
	@echo "  Stacks (apply order): $(ORDER)"
	@echo "  STACK=<name>   run a single stack, e.g. make up STACK=dns"
	@echo "  AUTO_APPROVE=1 skip confirmation prompts on up/destroy"

check-vars:
	@test -f $(ENV)/dns/terraform.tfvars || { echo "Missing $(ENV)/dns/terraform.tfvars (copy terraform.tfvars.example and set zone_name)"; exit 1; }

init: ## terraform init in each stack
	@for s in $(or $(STACK),$(ALL_STACKS)); do $(call tf,$$s,init -input=false) || exit 1; done

plan: check-vars ## Plan each stack (later stacks need upstream stacks applied first)
	@for s in $(UP_STACKS); do echo "==> $$s"; $(call tf,$$s,init -input=false >/dev/null) && $(call tf,$$s,plan -input=false $(COMMON_VARS)) || exit 1; done

up: check-vars ## Create/update the host: network -> compute -> dns -> inventory
	@for s in $(UP_STACKS); do echo "==> $$s"; $(call tf,$$s,init -input=false >/dev/null) && $(call tf,$$s,apply $(COMMON_VARS) $(APPROVE_FLAG)) || exit 1; done
	@echo; echo "Inventory: ansible/inventory/hosts.yml"; $(call tf,compute,output -raw ssm_session_command); echo

destroy: check-vars ## Tear down in reverse order (DELETES the telemetry volume)
	@for s in $(DOWN_STACKS); do echo "==> $$s"; $(call tf,$$s,init -input=false >/dev/null) && $(call tf,$$s,destroy $(COMMON_VARS) $(APPROVE_FLAG)) || exit 1; done

fmt: ## terraform fmt -recursive
	terraform -chdir=terraform fmt -recursive

validate: ## terraform validate in each stack
	@for s in $(ALL_STACKS); do echo "==> $$s"; $(call tf,$$s,init -backend=false -input=false >/dev/null) && $(call tf,$$s,validate) || exit 1; done

inventory: ## Print the generated Ansible inventory
	@$(call tf,inventory,output -raw ansible_inventory)

ssm: ## Open a shell on the host via SSM
	@$$($(call tf,compute,output -raw ssm_session_command))

$(VENV)/.installed: $(ANSIBLE_DIR)/requirements.txt
	python3 -m venv $(VENV)
	$(VENV)/bin/pip install --quiet --upgrade pip
	$(VENV)/bin/pip install --quiet -r $(ANSIBLE_DIR)/requirements.txt
	@touch $@

venv: $(VENV)/.installed ## Create .venv with the pinned Python deps

deps: venv ## Install Ansible collections
	$(VENV)/bin/ansible-galaxy collection install -r $(ANSIBLE_DIR)/requirements.yml -p $(ANSIBLE_DIR)/collections

configure: $(VENV)/.installed ## Run the Ansible playbook (ANSIBLE_ARGS="--tags base --check" etc.)
	cd $(ANSIBLE_DIR) && ansible-playbook site.yml $(ANSIBLE_ARGS)

check: $(VENV)/.installed ## Dry-run the playbook (--check --diff)
	cd $(ANSIBLE_DIR) && ansible-playbook site.yml --check --diff $(ANSIBLE_ARGS)

ping: $(VENV)/.installed ## Ansible ping over SSM
	cd $(ANSIBLE_DIR) && ansible shortener -m ping $(ANSIBLE_ARGS)

app-test: ## Run the app's tests in a Rust container (no local toolchain needed)
	docker run --rm -v "$(CURDIR)/app":/src -w /src -v shortener-cargo:/usr/local/cargo/registry rust:1-slim-bookworm cargo test

app-image: ## Build the arm64 app image (TAG=dev)
	docker buildx build --platform linux/arm64 --load -t shortener:$(or $(TAG),dev) app

registry-up: ## Create the ECR repo + GitHub OIDC role (not part of make up)
	@$(MAKE) --no-print-directory up STACK=registry

registry-destroy: ## Delete the ECR repo (and its images) + OIDC role
	@$(MAKE) --no-print-directory destroy STACK=registry

secrets-up: ## Create SSM parameters (placeholders; set real values yourself)
	@$(MAKE) --no-print-directory up STACK=secrets

secrets-destroy: ## Delete the SSM parameters (and the values you set)
	@$(MAKE) --no-print-directory destroy STACK=secrets

deploy: venv ## Deploy the app (APP_TAG=shortener-<sha> overrides group_vars/all.yml)
	cd $(ANSIBLE_DIR) && ansible-playbook site.yml --tags app $(if $(APP_TAG),-e app_image_tag=$(APP_TAG),) $(ANSIBLE_ARGS)
