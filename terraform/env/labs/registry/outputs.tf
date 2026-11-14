output "repository_url" {
  value = module.ecr.repository_url
}

output "repository_name" {
  value = module.ecr.repository_name
}

output "ci_role_arn" {
  description = "Set as the AWS_ROLE_ARN repository variable in GitHub."
  value       = module.github_oidc.role_arn
}

output "github_variables" {
  description = "Commands to configure the GitHub repository variables the workflow reads."
  value       = <<-EOT
    gh variable set AWS_ROLE_ARN   --body '${module.github_oidc.role_arn}'
    gh variable set AWS_REGION     --body '${var.region}'
    gh variable set ECR_REPOSITORY --body '${module.ecr.repository_name}'
  EOT
}
