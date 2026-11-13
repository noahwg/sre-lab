variable "name" {
  description = "Name prefix / Name tag for compute resources."
  type        = string
}

variable "subnet_id" {
  description = "Subnet to launch the instance in."
  type        = string
}

variable "availability_zone" {
  description = "AZ of the subnet; the telemetry volume is created here."
  type        = string
}

variable "security_group_ids" {
  description = "Security groups to attach to the instance."
  type        = list(string)
}

variable "instance_type" {
  description = "Must be an arm64 (Graviton) type, since the AMI is arm64."
  type        = string
  default     = "t4g.small"
}

variable "root_volume_size_gb" {
  description = "Root volume size (OS, container images, Postgres if kept on root)."
  type        = number
  default     = 20
}

variable "telemetry_volume_size_gb" {
  description = "Size of the dedicated EBS volume for logs, metrics, and traces."
  type        = number
  default     = 50
}

variable "telemetry_volume_type" {
  type    = string
  default = "gp3"
}

variable "telemetry_device_name" {
  description = "Requested device name. On Nitro instances it appears as /dev/nvmeXn1, so Ansible should resolve it by volume ID."
  type        = string
  default     = "/dev/sdf"
}
