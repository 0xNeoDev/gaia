CREATE TYPE "public"."proposalStatus" AS ENUM('created', 'executed', 'failed', 'expired');--> statement-breakpoint
CREATE TYPE "public"."proposalTypes" AS ENUM('publish_edit', 'add_member', 'remove_member', 'add_editor', 'remove_editor', 'add_subspace', 'remove_subspace');--> statement-breakpoint
ALTER TABLE "proposals" RENAME COLUMN "proposal_id" TO "id";--> statement-breakpoint
DROP INDEX "proposals_space_time_idx";--> statement-breakpoint
ALTER TABLE "proposals" ADD COLUMN "proposal_type" "proposalTypes" NOT NULL;--> statement-breakpoint
ALTER TABLE "proposals" ADD COLUMN "status" "proposalStatus" DEFAULT 'created' NOT NULL;--> statement-breakpoint
ALTER TABLE "proposals" ADD COLUMN "content_uri" text;--> statement-breakpoint
ALTER TABLE "proposals" ADD COLUMN "address" varchar(42);--> statement-breakpoint
ALTER TABLE "proposals" ADD COLUMN "created_at_block" bigint NOT NULL;--> statement-breakpoint
CREATE INDEX "proposals_status_idx" ON "proposals" USING btree ("status");--> statement-breakpoint
CREATE INDEX "proposals_type_idx" ON "proposals" USING btree ("proposal_type");--> statement-breakpoint
CREATE INDEX "proposals_address_idx" ON "proposals" USING btree ("address");--> statement-breakpoint
CREATE INDEX "proposals_space_status_idx" ON "proposals" USING btree ("space_id","status");--> statement-breakpoint
CREATE INDEX "proposals_space_type_idx" ON "proposals" USING btree ("space_id","proposal_type");--> statement-breakpoint
ALTER TABLE "proposals" DROP COLUMN "pluginAddress";--> statement-breakpoint
ALTER TABLE "proposals" DROP COLUMN "dao_address";