# Upstream stack: network (apply it first).
data "terraform_remote_state" "network" {
  backend = "local"
  config = {
    path = "${path.module}/../network/terraform.tfstate"
  }
}

module "compute" {
  source = "../../../modules/compute"

  name                     = var.name
  subnet_id                = data.terraform_remote_state.network.outputs.subnet_id
  availability_zone        = data.terraform_remote_state.network.outputs.availability_zone
  security_group_ids       = [data.terraform_remote_state.network.outputs.security_group_id]
  instance_type            = var.instance_type
  root_volume_size_gb      = var.root_volume_size_gb
  telemetry_volume_size_gb = var.telemetry_volume_size_gb
}
