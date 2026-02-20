from pydantic import BaseModel, ConfigDict


class Pool(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    url: str
    username: str
    password: str


class PoolGroup(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    name: str
    quota: int = 1
    pools: list[Pool]
