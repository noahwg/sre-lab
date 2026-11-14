variable "name" {
  description = "Name prefix for the IAM role."
  type        = string
}

variable "github_repository" {
  description = "GitHub repository allowed to assume the role, as owner/repo."
  type        = string
}

variable "github_refs" {
  description = "Git refs allowed to assume the role (pull requests are deliberately excluded)."
  type        = list(string)
  default     = ["refs/heads/main"]
}

variable "ecr_repository_arn" {
  description = "ECR repository the role may push to."
  type        = string
}

variable "existing_oidc_provider_arn" {
  description = "ARN of the account's existing GitHub OIDC provider. null creates one (an account can have only one)."
  type        = string
  default     = null
}
