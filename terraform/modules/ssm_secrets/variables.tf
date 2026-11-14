variable "parameters" {
  description = <<-EOT
    SecureString parameters to create, keyed by full path (e.g. /sre-lab/shortener/api_key).
    Terraform creates each with a placeholder and never touches the value again:
    set the real value in the console or with `aws ssm put-parameter --overwrite`.
  EOT
  type = map(object({
    description = string
  }))
}

variable "placeholder" {
  description = "Initial value. Deliberately shorter than any real secret so a forgotten update fails loudly."
  type        = string
  default     = "REPLACE_ME"
}
