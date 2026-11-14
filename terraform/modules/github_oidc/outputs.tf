output "role_arn" {
  description = "Role for GitHub Actions to assume (set as the AWS_ROLE_ARN repository variable)."
  value       = aws_iam_role.ci.arn
}
