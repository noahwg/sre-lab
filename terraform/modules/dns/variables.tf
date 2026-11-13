variable "zone_name" {
  description = "Existing Route 53 public hosted zone, e.g. example.com (no trailing dot)."
  type        = string
}

variable "record_names" {
  description = "Records to point at the host. \"@\" is the apex; others are subdomains (e.g. \"grafana\")."
  type        = list(string)
  default     = ["@"]
}

variable "ip_address" {
  description = "IPv4 address the records resolve to (the host's Elastic IP)."
  type        = string
}

variable "ttl" {
  type    = number
  default = 300
}
