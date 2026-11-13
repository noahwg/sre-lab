variable "name" {
  description = "Name prefix for network resources."
  type        = string
}

variable "availability_zone" {
  description = "AZ for the public subnet. The compute module's EBS volumes must live in the same AZ."
  type        = string
}

variable "vpc_cidr" {
  description = "CIDR block for the VPC."
  type        = string
  default     = "10.20.0.0/16"
}

variable "public_subnet_cidr" {
  description = "CIDR block for the public subnet."
  type        = string
  default     = "10.20.1.0/24"
}

variable "web_ingress_cidrs" {
  description = "CIDRs allowed to reach ports 80/443. Open to the world by default since this is a public URL shortener."
  type        = list(string)
  default     = ["0.0.0.0/0"]
}
