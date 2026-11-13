variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes resource names and is the Ansible host name."
  type        = string
  default     = "sre-lab"
}

variable "instance_type" {
  type    = string
  default = "t4g.small"
}

variable "root_volume_size_gb" {
  type    = number
  default = 20
}

variable "telemetry_volume_size_gb" {
  type    = number
  default = 50
}
