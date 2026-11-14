module "secrets" {
  source = "../../../modules/ssm_secrets"

  parameters = {
    "/${var.name}/shortener/api_key" = {
      description = "API key for the shortener's management endpoints (x-api-key). Set the value outside Terraform."
    }
  }
}
