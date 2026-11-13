output "instance_id" {
  value = aws_instance.host.id
}

output "public_ip" {
  value = aws_eip.host.public_ip
}

output "private_ip" {
  value = aws_instance.host.private_ip
}

output "telemetry_volume_id" {
  value = aws_ebs_volume.telemetry.id
}

output "telemetry_device_name" {
  value = var.telemetry_device_name
}

output "ssm_transfer_bucket" {
  value = aws_s3_bucket.ssm_transfer.id
}
