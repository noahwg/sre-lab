ENV          := terraform/env/labs
ORDER        := network compute dns inventory
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
.PHONY: help up destroy plan init fmt validate inventory ssm check-vars

help: ## Show targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'
	@echo
	@echo "  Stacks (apply order): $(ORDER)"
	@echo "  STACK=<name>   run a single stack, e.g. make up STACK=dns"
	@echo "  AUTO_APPROVE=1 skip confirmation prompts on up/destroy"

check-vars:
	@test -f $(ENV)/dns/terraform.tfvars || { echo "Missing $(ENV)/dns/terraform.tfvars (copy terraform.tfvars.example and set zone_name)"; exit 1; }

init: ## terraform init in each stack
	@for s in $(UP_STACKS); do $(call tf,$$s,init -input=false) || exit 1; done

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
	@for s in $(ORDER); do echo "==> $$s"; $(call tf,$$s,init -backend=false -input=false >/dev/null) && $(call tf,$$s,validate) || exit 1; done

inventory: ## Print the generated Ansible inventory
	@$(call tf,inventory,output -raw ansible_inventory)

ssm: ## Open a shell on the host via SSM
	@$$($(call tf,compute,output -raw ssm_session_command))
