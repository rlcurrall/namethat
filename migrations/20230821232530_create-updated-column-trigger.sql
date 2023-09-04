-- -----------------------------------------------------------------------------
-- Create updated column trigger
-- -----------------------------------------------------------------------------

create or replace function mod_datetime()
    returns trigger as
$$
begin
    new.updated = now();
    return new;
end;
$$ language plpgsql;
