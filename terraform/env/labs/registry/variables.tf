variable "region" {
  type    = string
  default = "us-east-1"
}

variable "name" {
  description = "Project name; prefixes resource names."
  type        = string
  default     = "sre-lab"
}

variable "github_repository" {
  description = "GitHub repository (owner/repo) whose Actions workflows may push images."
  type        = string
  default     = "noahwg/sre-lab"
}

variable "github_refs" {
  description = "Git refs allowed to push. Pull requests build but never push."
  type        = list(string)
  default     = ["refs/heads/main"]
}

variable "existing_oidc_provider_arn" {
  description = "Set if this account already has the GitHub OIDC provider (there can be only one). null creates it."
  type        = string
  default     = null
}

variable "keep_image_count" {
  description = "Images to keep across ALL images in the shared repo (tags look like <image>-<sha>)."
  type        = number
  default     = 30
}
