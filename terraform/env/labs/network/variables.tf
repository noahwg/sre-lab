variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes resource names and is the Ansible host name."
  type        = string
  default     = "sre-lab"
}

variable "web_ingress_cidrs" {
  description = "CIDRs allowed to reach 80/443."
  type        = list(string)
  default     = ["0.0.0.0/0"]
}
