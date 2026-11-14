output "repository_url" {
  description = "Registry URL without a tag, e.g. 123456789012.dkr.ecr.us-east-1.amazonaws.com/sre-lab."
  value       = aws_ecr_repository.this.repository_url
}

output "repository_arn" {
  value = aws_ecr_repository.this.arn
}

output "repository_name" {
  value = aws_ecr_repository.this.name
}
