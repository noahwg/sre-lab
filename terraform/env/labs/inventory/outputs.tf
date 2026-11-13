output "ansible_inventory" {
  description = "Ansible inventory (YAML). Print with `make inventory`."
  value       = local.ansible_inventory
}
