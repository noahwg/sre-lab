output "parameter_names" {
  value = module.secrets.parameter_names
}

output "set_value_hint" {
  description = "How to populate the placeholder values."
  value       = <<-EOT
    Set each real value in the console (Systems Manager > Parameter Store > Edit), or:
      aws ssm put-parameter --region ${var.region} --name /${var.name}/shortener/api_key \
        --type SecureString --overwrite --value "$(openssl rand -hex 24)"
  EOT
}
