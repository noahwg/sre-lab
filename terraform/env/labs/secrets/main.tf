module "secrets" {
  source = "../../../modules/ssm_secrets"

  parameters = {
    "/${var.name}/shortener/api_key" = {
      description = "API key for the shortener's management endpoints (x-api-key). Set the value outside Terraform."
    }
    "/${var.name}/postgres/owner_password" = {
      description = "Password for the Postgres owner role (superuser; runs migrations). Set the value outside Terraform."
    }
    "/${var.name}/postgres/app_password" = {
      description = "Password for the shortener's restricted Postgres runtime role. Set the value outside Terraform."
    }
    "/${var.name}/monitoring/grafana_admin_password" = {
      description = "Admin password for Grafana. Set the value outside Terraform."
    }
  }
}
