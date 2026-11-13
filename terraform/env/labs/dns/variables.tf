variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes resource names and is the Ansible host name."
  type        = string
  default     = "sre-lab"
}

variable "zone_name" {
  description = "Existing Route 53 public hosted zone, e.g. example.com."
  type        = string
}

variable "record_names" {
  description = "DNS records to point at the host (\"@\" = apex). The first entry is the primary FQDN."
  type        = list(string)
  default     = ["@"]
}
