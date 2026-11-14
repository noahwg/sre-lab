variable "repository_name" {
  description = "ECR repository name, e.g. sre-lab."
  type        = string
}

variable "keep_image_count" {
  description = "Number of most recent images to keep; older ones expire."
  type        = number
  default     = 30
}

variable "force_delete" {
  description = "Allow terraform destroy to delete the repository even if it still holds images."
  type        = bool
  default     = false
}
