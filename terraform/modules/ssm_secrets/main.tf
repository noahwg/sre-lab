resource "aws_ssm_parameter" "this" {
  for_each = var.parameters

  name        = each.key
  description = each.value.description
  type        = "SecureString" # encrypted with the AWS-managed aws/ssm key (no KMS charge)
  value       = var.placeholder

  lifecycle {
    # The real value is set outside Terraform, so it never lands in state and
    # later applies don't reset it to the placeholder.
    ignore_changes = [value]
  }
}
