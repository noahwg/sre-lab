variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes parameter paths (/<name>/...)."
  type        = string
  default     = "sre-lab"
}
