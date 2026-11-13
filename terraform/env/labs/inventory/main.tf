# Upstream stacks: compute and dns (apply them first).
data "terraform_remote_state" "compute" {
  backend = "local"
  config = {
    path = "${path.module}/../compute/terraform.tfstate"
  }
}

data "terraform_remote_state" "dns" {
  backend = "local"
  config = {
    path = "${path.module}/../dns/terraform.tfstate"
  }
}

locals {
  compute = data.terraform_remote_state.compute.outputs
  dns     = data.terraform_remote_state.dns.outputs

  ansible_inventory = templatefile("${path.module}/templates/inventory.yml.tftpl", {
    host_name             = var.name
    instance_id           = local.compute.instance_id
    region                = var.region
    ssm_bucket            = local.compute.ssm_transfer_bucket
    public_ip             = local.compute.public_ip
    primary_fqdn          = local.dns.primary_fqdn
    fqdns                 = local.dns.fqdns
    telemetry_volume_id   = local.compute.telemetry_volume_id
    telemetry_mount_point = var.telemetry_mount_point
  })
}

resource "local_file" "ansible_inventory" {
  count = var.write_inventory_file ? 1 : 0

  filename             = "${path.module}/../../../../ansible/inventory/hosts.yml"
  content              = local.ansible_inventory
  file_permission      = "0644"
  directory_permission = "0755"
}
