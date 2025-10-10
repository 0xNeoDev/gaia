CREATE TYPE "public"."dataTypes" AS ENUM('String', 'Number', 'Boolean', 'Time', 'Point', 'Relation');--> statement-breakpoint
CREATE TYPE "public"."spaceTypes" AS ENUM('Personal', 'Public');--> statement-breakpoint
CREATE TABLE "editors" (
	"address" text NOT NULL,
	"space_id" uuid NOT NULL,
	CONSTRAINT "editors_address_space_id_pk" PRIMARY KEY("address","space_id")
);
--> statement-breakpoint
CREATE TABLE "entities" (
	"id" uuid PRIMARY KEY NOT NULL,
	"created_at" text NOT NULL,
	"created_at_block" text NOT NULL,
	"updated_at" text NOT NULL,
	"updated_at_block" text NOT NULL
);
--> statement-breakpoint
CREATE TABLE "ipfs_cache" (
	"id" serial NOT NULL,
	"json" jsonb,
	"uri" text NOT NULL,
	"is_errored" boolean DEFAULT false NOT NULL,
	"block" text NOT NULL,
	"space" uuid NOT NULL,
	CONSTRAINT "ipfs_cache_uri_unique" UNIQUE("uri")
);
--> statement-breakpoint
CREATE TABLE "members" (
	"address" text NOT NULL,
	"space_id" uuid NOT NULL,
	CONSTRAINT "members_address_space_id_pk" PRIMARY KEY("address","space_id")
);
--> statement-breakpoint
CREATE TABLE "meta" (
	"id" text PRIMARY KEY NOT NULL,
	"cursor" text NOT NULL,
	"block_number" text NOT NULL
);
--> statement-breakpoint
CREATE TABLE "properties" (
	"id" uuid PRIMARY KEY NOT NULL,
	"type" "dataTypes" NOT NULL
);
--> statement-breakpoint
CREATE TABLE "proposals" (
	"proposal_id" uuid PRIMARY KEY NOT NULL,
	"pluginAddress" varchar(42) NOT NULL,
	"creator" varchar(42) NOT NULL,
	"start_time" bigint NOT NULL,
	"end_time" bigint NOT NULL,
	"dao_address" text NOT NULL,
	"space_id" uuid NOT NULL
);
--> statement-breakpoint
CREATE TABLE "raw_actions" (
	"id" serial PRIMARY KEY NOT NULL,
	"action_type" bigint NOT NULL,
	"action_version" bigint NOT NULL,
	"sender" varchar(42) NOT NULL,
	"entity" uuid NOT NULL,
	"group_id" uuid,
	"space_pov" uuid NOT NULL,
	"metadata" "bytea",
	"block_number" bigint NOT NULL,
	"block_timestamp" timestamp with time zone NOT NULL,
	"tx_hash" varchar(66) NOT NULL
);
--> statement-breakpoint
CREATE TABLE "relations" (
	"id" uuid PRIMARY KEY NOT NULL,
	"entity_id" uuid NOT NULL,
	"type_id" uuid NOT NULL,
	"from_entity_id" uuid NOT NULL,
	"from_space_id" uuid,
	"from_version_id" uuid,
	"to_entity_id" uuid NOT NULL,
	"to_space_id" uuid,
	"to_version_id" uuid,
	"position" text,
	"space_id" uuid NOT NULL,
	"verified" boolean
);
--> statement-breakpoint
CREATE TABLE "spaces" (
	"id" uuid PRIMARY KEY NOT NULL,
	"type" "spaceTypes" NOT NULL,
	"dao_address" text NOT NULL,
	"space_address" text NOT NULL,
	"main_voting_address" text,
	"membership_address" text,
	"personal_address" text
);
--> statement-breakpoint
CREATE TABLE "subspaces" (
	"parent_space_id" uuid NOT NULL,
	"child_space_id" uuid NOT NULL,
	CONSTRAINT "subspaces_parent_space_id_child_space_id_pk" PRIMARY KEY("parent_space_id","child_space_id")
);
--> statement-breakpoint
CREATE TABLE "user_votes" (
	"id" serial PRIMARY KEY NOT NULL,
	"user_id" varchar(42) NOT NULL,
	"entity_id" uuid NOT NULL,
	"space_id" uuid NOT NULL,
	"vote_type" smallint NOT NULL,
	"voted_at" timestamp with time zone NOT NULL,
	CONSTRAINT "user_votes_user_entity_space_unique" UNIQUE("user_id","entity_id","space_id")
);
--> statement-breakpoint
CREATE TABLE "values" (
	"id" text PRIMARY KEY NOT NULL,
	"property_id" uuid NOT NULL,
	"entity_id" uuid NOT NULL,
	"space_id" uuid NOT NULL,
	"string" text,
	"boolean" boolean,
	"number" numeric,
	"point" text,
	"time" text,
	"language" text,
	"unit" text
);
--> statement-breakpoint
CREATE TABLE "votes_count" (
	"id" serial PRIMARY KEY NOT NULL,
	"entity_id" uuid NOT NULL,
	"space_id" uuid NOT NULL,
	"upvotes" bigint DEFAULT 0 NOT NULL,
	"downvotes" bigint DEFAULT 0 NOT NULL,
	CONSTRAINT "votes_count_entity_space_unique" UNIQUE("entity_id","space_id")
);
--> statement-breakpoint
ALTER TABLE "editors" ADD CONSTRAINT "editors_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "members" ADD CONSTRAINT "members_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "proposals" ADD CONSTRAINT "proposals_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_type_id_properties_id_fk" FOREIGN KEY ("type_id") REFERENCES "public"."properties"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_from_entity_id_entities_id_fk" FOREIGN KEY ("from_entity_id") REFERENCES "public"."entities"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_from_space_id_spaces_id_fk" FOREIGN KEY ("from_space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_to_entity_id_entities_id_fk" FOREIGN KEY ("to_entity_id") REFERENCES "public"."entities"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_to_space_id_spaces_id_fk" FOREIGN KEY ("to_space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "relations" ADD CONSTRAINT "relations_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "subspaces" ADD CONSTRAINT "subspaces_parent_space_id_spaces_id_fk" FOREIGN KEY ("parent_space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "subspaces" ADD CONSTRAINT "subspaces_child_space_id_spaces_id_fk" FOREIGN KEY ("child_space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "values" ADD CONSTRAINT "values_property_id_properties_id_fk" FOREIGN KEY ("property_id") REFERENCES "public"."properties"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "values" ADD CONSTRAINT "values_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "values" ADD CONSTRAINT "values_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "editors_space_id_idx" ON "editors" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "entities_updated_at_idx" ON "entities" USING btree ("updated_at");--> statement-breakpoint
CREATE INDEX "entities_updated_at_id_idx" ON "entities" USING btree ("updated_at","id");--> statement-breakpoint
CREATE INDEX "members_space_id_idx" ON "members" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "properties_type_idx" ON "properties" USING btree ("type");--> statement-breakpoint
CREATE INDEX "proposals_space_id_idx" ON "proposals" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "proposals_creator_idx" ON "proposals" USING btree ("creator");--> statement-breakpoint
CREATE INDEX "proposals_start_time_idx" ON "proposals" USING btree ("start_time");--> statement-breakpoint
CREATE INDEX "proposals_end_time_idx" ON "proposals" USING btree ("end_time");--> statement-breakpoint
CREATE INDEX "proposals_space_time_idx" ON "proposals" USING btree ("space_id","start_time","end_time");--> statement-breakpoint
CREATE INDEX "relations_entity_id_idx" ON "relations" USING btree ("entity_id");--> statement-breakpoint
CREATE INDEX "relations_type_id_idx" ON "relations" USING btree ("type_id");--> statement-breakpoint
CREATE INDEX "relations_from_entity_id_idx" ON "relations" USING btree ("from_entity_id");--> statement-breakpoint
CREATE INDEX "relations_to_entity_id_idx" ON "relations" USING btree ("to_entity_id");--> statement-breakpoint
CREATE INDEX "relations_space_id_idx" ON "relations" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "relations_space_from_to_idx" ON "relations" USING btree ("space_id","from_entity_id","to_entity_id");--> statement-breakpoint
CREATE INDEX "relations_space_type_idx" ON "relations" USING btree ("space_id","type_id");--> statement-breakpoint
CREATE INDEX "relations_to_entity_space_idx" ON "relations" USING btree ("to_entity_id","space_id");--> statement-breakpoint
CREATE INDEX "relations_from_entity_space_idx" ON "relations" USING btree ("from_entity_id","space_id");--> statement-breakpoint
CREATE INDEX "relations_entity_type_space_idx" ON "relations" USING btree ("entity_id","type_id","space_id");--> statement-breakpoint
CREATE INDEX "relations_type_from_to_idx" ON "relations" USING btree ("type_id","from_entity_id","to_entity_id");--> statement-breakpoint
CREATE INDEX "subspaces_parent_space_id_idx" ON "subspaces" USING btree ("parent_space_id");--> statement-breakpoint
CREATE INDEX "subspaces_child_space_id_idx" ON "subspaces" USING btree ("child_space_id");--> statement-breakpoint
CREATE INDEX "idx_user_votes_user_entity_space" ON "user_votes" USING btree ("user_id","entity_id","space_id");--> statement-breakpoint
CREATE INDEX "values_property_id_idx" ON "values" USING btree ("property_id");--> statement-breakpoint
CREATE INDEX "values_entity_id_idx" ON "values" USING btree ("entity_id");--> statement-breakpoint
CREATE INDEX "values_space_id_idx" ON "values" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "values_text_idx" ON "values" USING btree ("string");--> statement-breakpoint
CREATE INDEX "values_number_idx" ON "values" USING btree ("number");--> statement-breakpoint
CREATE INDEX "values_point_idx" ON "values" USING btree ("point");--> statement-breakpoint
CREATE INDEX "values_boolean_idx" ON "values" USING btree ("boolean");--> statement-breakpoint
CREATE INDEX "values_time_idx" ON "values" USING btree ("time");--> statement-breakpoint
CREATE INDEX "values_entity_property_idx" ON "values" USING btree ("entity_id","property_id");--> statement-breakpoint
CREATE INDEX "values_entity_space_idx" ON "values" USING btree ("entity_id","space_id");--> statement-breakpoint
CREATE INDEX "values_property_space_idx" ON "values" USING btree ("property_id","space_id");--> statement-breakpoint
CREATE INDEX "values_entity_property_space_idx" ON "values" USING btree ("entity_id","property_id","space_id");--> statement-breakpoint
CREATE INDEX "values_space_text_idx" ON "values" USING btree ("space_id","string");--> statement-breakpoint
CREATE INDEX "values_language_idx" ON "values" USING btree ("language");--> statement-breakpoint
CREATE INDEX "values_unit_idx" ON "values" USING btree ("unit");--> statement-breakpoint
CREATE INDEX "idx_votes_count_space" ON "votes_count" USING btree ("space_id");--> statement-breakpoint
CREATE INDEX "idx_votes_count_entity_space" ON "votes_count" USING btree ("entity_id","space_id");