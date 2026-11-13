# Upstream stack: compute (apply it first).
data "terraform_remote_state" "compute" {
  backend = "local"
  config = {
    path = "${path.module}/../compute/terraform.tfstate"
  }
}

module "dns" {
  source = "../../../modules/dns"

  zone_name    = var.zone_name
  record_names = var.record_names
  ip_address   = data.terraform_remote_state.compute.outputs.public_ip
}
