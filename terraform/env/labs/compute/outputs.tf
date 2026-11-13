output "instance_id" {
  value = module.compute.instance_id
}

output "public_ip" {
  value = module.compute.public_ip
}

output "telemetry_volume_id" {
  value = module.compute.telemetry_volume_id
}

output "ssm_transfer_bucket" {
  value = module.compute.ssm_transfer_bucket
}

output "ssm_session_command" {
  description = "Interactive shell on the host (no SSH needed)."
  value       = "aws ssm start-session --region ${var.region} --target ${module.compute.instance_id}"
}
