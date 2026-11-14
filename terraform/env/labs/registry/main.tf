module "ecr" {
  source = "../../../modules/ecr"

  repository_name  = var.name
  keep_image_count = var.keep_image_count
  # Lab convenience: `destroy` removes the repo even though it holds images.
  force_delete = true
}

module "github_oidc" {
  source = "../../../modules/github_oidc"

  name                       = var.name
  github_repository          = var.github_repository
  github_refs                = var.github_refs
  ecr_repository_arn         = module.ecr.repository_arn
  existing_oidc_provider_arn = var.existing_oidc_provider_arn
}
