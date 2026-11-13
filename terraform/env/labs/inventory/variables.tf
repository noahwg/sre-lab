variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes resource names and is the Ansible host name."
  type        = string
  default     = "sre-lab"
}

variable "telemetry_mount_point" {
  description = "Where Ansible should mount the telemetry volume."
  type        = string
  default     = "/var/lib/telemetry"
}

variable "write_inventory_file" {
  description = "Also write the inventory to ansible/inventory/hosts.yml."
  type        = bool
  default     = true
}
