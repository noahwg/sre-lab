output "parameter_names" {
  description = "Full names of the created parameters."
  value       = [for p in aws_ssm_parameter.this : p.name]
}

output "parameter_arns" {
  value = { for k, p in aws_ssm_parameter.this : k => p.arn }
}
