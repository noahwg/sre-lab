data "aws_caller_identity" "current" {}
data "aws_partition" "current" {}

# Latest Amazon Linux 2023 (arm64, for Graviton). SSM agent is preinstalled.
data "aws_ssm_parameter" "al2023_arm64" {
  name = "/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64"
}

# ---------------------------------------------------------------------------
# IAM: SSM Session Manager + the S3 bucket Ansible's aws_ssm connection uses
# ---------------------------------------------------------------------------

data "aws_iam_policy_document" "assume_ec2" {
  statement {
    actions = ["sts:AssumeRole"]
    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "host" {
  name_prefix        = "${var.name}-host-"
  assume_role_policy = data.aws_iam_policy_document.assume_ec2.json
}

resource "aws_iam_role_policy_attachment" "ssm_core" {
  role       = aws_iam_role.host.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/AmazonSSMManagedInstanceCore"
}

resource "aws_iam_instance_profile" "host" {
  name_prefix = "${var.name}-host-"
  role        = aws_iam_role.host.name
}

# Ansible's community.aws.aws_ssm connection plugin moves files through S3.
resource "aws_s3_bucket" "ssm_transfer" {
  bucket        = "${var.name}-ssm-transfer-${data.aws_caller_identity.current.account_id}"
  force_destroy = true
}

resource "aws_s3_bucket_public_access_block" "ssm_transfer" {
  bucket                  = aws_s3_bucket.ssm_transfer.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_server_side_encryption_configuration" "ssm_transfer" {
  bucket = aws_s3_bucket.ssm_transfer.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "ssm_transfer" {
  bucket = aws_s3_bucket.ssm_transfer.id

  rule {
    id     = "expire-transfers"
    status = "Enabled"
    filter {}
    expiration {
      days = 1
    }
    abort_incomplete_multipart_upload {
      days_after_initiation = 1
    }
  }
}

data "aws_iam_policy_document" "ssm_transfer" {
  statement {
    actions   = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject"]
    resources = ["${aws_s3_bucket.ssm_transfer.arn}/*"]
  }
  statement {
    actions   = ["s3:ListBucket", "s3:GetBucketLocation"]
    resources = [aws_s3_bucket.ssm_transfer.arn]
  }
}

resource "aws_iam_role_policy" "ssm_transfer" {
  name   = "ssm-transfer-bucket"
  role   = aws_iam_role.host.id
  policy = data.aws_iam_policy_document.ssm_transfer.json
}

# ---------------------------------------------------------------------------
# Instance
# ---------------------------------------------------------------------------

resource "aws_instance" "host" {
  ami                    = nonsensitive(data.aws_ssm_parameter.al2023_arm64.value)
  instance_type          = var.instance_type
  subnet_id              = var.subnet_id
  vpc_security_group_ids = var.security_group_ids
  iam_instance_profile   = aws_iam_instance_profile.host.name

  # No key pair on purpose: SSM only.
  associate_public_ip_address = false

  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required" # IMDSv2 only
    http_put_response_hop_limit = 2          # containers on the host can reach IMDS
  }

  root_block_device {
    volume_type           = "gp3"
    volume_size           = var.root_volume_size_gb
    encrypted             = true
    delete_on_termination = true
  }

  tags = { Name = var.name }

  lifecycle {
    # Don't replace the host (and re-provision it) every time a new AMI ships.
    ignore_changes = [ami]
  }
}

resource "aws_eip" "host" {
  domain   = "vpc"
  instance = aws_instance.host.id

  tags = { Name = var.name }
}

# ---------------------------------------------------------------------------
# Telemetry volume: separate from the root disk so logs/metrics/traces survive
# instance replacement and can be sized, snapshotted, and resized independently.
# ---------------------------------------------------------------------------

resource "aws_ebs_volume" "telemetry" {
  availability_zone = var.availability_zone
  type              = var.telemetry_volume_type
  size              = var.telemetry_volume_size_gb
  encrypted         = true

  tags = { Name = "${var.name}-telemetry" }
}

resource "aws_volume_attachment" "telemetry" {
  device_name = var.telemetry_device_name
  volume_id   = aws_ebs_volume.telemetry.id
  instance_id = aws_instance.host.id

  stop_instance_before_detaching = true
}
